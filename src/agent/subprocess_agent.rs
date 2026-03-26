use std::process::Stdio;
use std::time::Duration;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt, BufWriter},
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
///
/// `stdin_rx`: when `Some`, the process gets a piped stdin and messages
/// received on this channel are written to it (used for interactive prompts).
pub async fn run(
    command:      String,
    args:         Vec<String>,
    project_name: String,
    tab_kind:     AgentKind,
    cwd:          String,
    messages:     Option<Vec<ConversationMessage>>,
    tx:           mpsc::UnboundedSender<AgentMessage>,
    cancel_rx:    oneshot::Receiver<()>,
    stdin_rx:     Option<mpsc::UnboundedReceiver<String>>,
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
    let replace = |text: String| AgentMessage::ReplaceLine {
        project_name: project_name.clone(),
        tab_kind: tab_kind.clone(),
        text,
    };
    let state_msg = |s: AgentState| AgentMessage::StateChange {
        project_name: project_name.clone(),
        tab_kind: tab_kind.clone(),
        new_state: s,
    };

    tx.send(state_msg(AgentState::Running)).ok();

    let stdin_stdio = if stdin_rx.is_some() { Stdio::piped() } else { Stdio::null() };

    let mut child = match Command::new(&command)
        .args(&args)
        .current_dir(&cwd)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .stdin(stdin_stdio)
        .spawn()
    {
        Ok(c) => c,
        Err(e) => {
            tx.send(chunk(format!("[error spawning {command}] {e}"), true)).ok();
            tx.send(state_msg(AgentState::Error)).ok();
            return;
        }
    };

    // Pipe stdin from UI → process
    if let Some(mut stdin_rx) = stdin_rx {
        if let Some(stdin) = child.stdin.take() {
            let mut writer = BufWriter::new(stdin);
            tokio::spawn(async move {
                while let Some(text) = stdin_rx.recv().await {
                    if writer.write_all(text.as_bytes()).await.is_err() { break; }
                    if writer.flush().await.is_err() { break; }
                }
            });
        }
    }

    // Drain stderr in a separate task
    let tx2   = tx.clone();
    let pn2   = project_name.clone();
    let kind2 = tab_kind.clone();
    if let Some(stderr) = child.stderr.take() {
        use tokio::io::{AsyncBufReadExt, BufReader};
        tokio::spawn(async move {
            let mut lines = BufReader::new(stderr).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                tx2.send(AgentMessage::Chunk {
                    project_name: pn2.clone(),
                    tab_kind:     kind2.clone(),
                    text:         format!("[stderr] {line}"),
                    is_newline:   true,
                }).ok();
            }
        });
    }

    // Stdout: chunk-based read with 50 ms timeout to flush partial lines
    // (permission prompts often lack a trailing newline)
    let mut stdout     = child.stdout.take().unwrap();
    let mut line_buf   = String::new();
    let mut partial_pending = false;   // true = last push was a partial (no \n yet)
    let mut cancel_rx  = std::pin::pin!(cancel_rx);
    let mut byte_buf   = [0u8; 4096];

    loop {
        let read_fut = stdout.read(&mut byte_buf);
        let timed    = tokio::time::timeout(Duration::from_millis(50), read_fut);

        tokio::select! {
            biased;
            _ = &mut cancel_rx => {
                child.kill().await.ok();
                tx.send(state_msg(AgentState::Idle)).ok();
                return;
            }
            result = timed => {
                match result {
                    // Timeout: flush buffered partial so prompts appear without waiting for \n
                    Err(_) => {
                        if !line_buf.is_empty() && !partial_pending {
                            tx.send(chunk(line_buf.clone(), true)).ok();
                            partial_pending = true;
                        }
                    }
                    Ok(Ok(0)) => break,   // EOF
                    Ok(Ok(n)) => {
                        let s = String::from_utf8_lossy(&byte_buf[..n]).to_string();
                        line_buf.push_str(&s);

                        // Drain all complete lines from the buffer
                        while let Some(pos) = line_buf.find('\n') {
                            let line = line_buf[..pos].trim_end_matches('\r').to_string();
                            if partial_pending {
                                // We already showed a partial; replace it with the full line
                                tx.send(replace(line)).ok();
                                partial_pending = false;
                            } else {
                                tx.send(chunk(line, true)).ok();
                            }
                            line_buf = line_buf[pos + 1..].to_string();
                        }
                        // Remaining bytes are a partial line — will flush on next timeout
                    }
                    Ok(Err(e)) => {
                        tx.send(chunk(format!("[read error] {e}"), true)).ok();
                        break;
                    }
                }
            }
        }
    }

    // Flush any remaining partial line at EOF
    if !line_buf.is_empty() {
        if partial_pending {
            tx.send(replace(line_buf)).ok();
        } else {
            tx.send(chunk(line_buf, true)).ok();
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
