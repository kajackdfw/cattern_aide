use std::process::Stdio;
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    process::Command,
    sync::{mpsc, oneshot},
};
use crate::state::agent::{AgentKind, AgentState};
use crate::state::project::ConversationMessage;
use super::AgentMessage;

/// Format conversation history as a single prompt string for CLIs that
/// don't natively support multi-turn messages (e.g. `claude -p`).
fn format_conversation(messages: &[ConversationMessage]) -> String {
    if messages.len() == 1 {
        return messages[0].content.clone();
    }
    let mut out = String::from("Continue this conversation:\n\n");
    for m in messages {
        let role = if m.role == "user" { "User" } else { "Assistant" };
        out.push_str(&format!("{}: {}\n\n", role, m.content));
    }
    out
}

/// `args` are the complete argument list when `messages` is `None`
/// (e.g. git commands), or the args *prefix* when `messages` is `Some`
/// (the formatted conversation is appended as the final argument).
pub async fn run(
    command:      String,
    args:         Vec<String>,
    project_name: String,
    tab_kind:     AgentKind,
    cwd:          String,
    messages:     Option<Vec<ConversationMessage>>,
    tx:           mpsc::UnboundedSender<AgentMessage>,
    cancel_rx:    oneshot::Receiver<()>,
) {
    let mut args = args;
    if let Some(msgs) = messages {
        args.push(format_conversation(&msgs));
    }
    let chunk = |text: String, nl: bool| AgentMessage::Chunk {
        project_name: project_name.clone(),
        tab_kind: tab_kind.clone(),
        text,
        is_newline: nl,
    };
    let state_msg = |s: AgentState| AgentMessage::StateChange {
        project_name: project_name.clone(),
        tab_kind: tab_kind.clone(),
        new_state: s,
    };

    tx.send(state_msg(AgentState::Running)).ok();

    let mut child = match Command::new(&command)
        .args(&args)
        .current_dir(&cwd)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(c) => c,
        Err(e) => {
            tx.send(chunk(format!("[error spawning {command}] {e}"), true)).ok();
            tx.send(state_msg(AgentState::Error)).ok();
            return;
        }
    };

    let stdout = BufReader::new(child.stdout.take().unwrap());
    let stderr = BufReader::new(child.stderr.take().unwrap());

    // Drain stderr in a separate task
    let tx2   = tx.clone();
    let pn2   = project_name.clone();
    let kind2 = tab_kind.clone();
    tokio::spawn(async move {
        let mut lines = stderr.lines();
        while let Ok(Some(line)) = lines.next_line().await {
            tx2.send(AgentMessage::Chunk {
                project_name: pn2.clone(),
                tab_kind:     kind2.clone(),
                text:         format!("[stderr] {line}"),
                is_newline:   true,
            }).ok();
        }
    });

    let mut stdout_lines = stdout.lines();
    let mut cancel_rx    = std::pin::pin!(cancel_rx);

    loop {
        tokio::select! {
            biased;
            _ = &mut cancel_rx => {
                child.kill().await.ok();
                tx.send(state_msg(AgentState::Idle)).ok();
                return;
            }
            line = stdout_lines.next_line() => {
                match line {
                    Ok(Some(l)) => { tx.send(chunk(l, true)).ok(); }
                    Ok(None)    => break,
                    Err(e)      => {
                        tx.send(chunk(format!("[read error] {e}"), true)).ok();
                        break;
                    }
                }
            }
        }
    }

    match child.wait().await {
        Ok(s) if s.success() => {
            tx.send(state_msg(AgentState::Idle)).ok();
        }
        Ok(s) => {
            tx.send(chunk(format!("[exited {}]", s.code().unwrap_or(-1)), true)).ok();
            tx.send(state_msg(AgentState::Error)).ok();
        }
        Err(e) => {
            tx.send(chunk(format!("[wait error] {e}"), true)).ok();
            tx.send(state_msg(AgentState::Error)).ok();
        }
    }
}
