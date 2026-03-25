use futures::StreamExt;
use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE};
use serde_json::Value;
use tokio::sync::{mpsc, oneshot};
use crate::state::agent::{AgentKind, AgentState};
use crate::state::project::ConversationMessage;
use super::AgentMessage;

pub async fn run(
    project_name: String,
    tab_kind:     AgentKind,
    messages:     Vec<ConversationMessage>,
    api_key:      String,
    model:        String,
    tx:           mpsc::UnboundedSender<AgentMessage>,
    cancel_rx:    oneshot::Receiver<()>,
) {
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

    let mut headers = HeaderMap::new();
    headers.insert(
        "x-api-key",
        HeaderValue::from_str(&api_key).unwrap_or_else(|_| HeaderValue::from_static("")),
    );
    headers.insert("anthropic-version", HeaderValue::from_static("2023-06-01"));
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

    let api_messages: Vec<serde_json::Value> = messages.iter()
        .map(|m| serde_json::json!({ "role": m.role, "content": m.content }))
        .collect();

    let body = serde_json::json!({
        "model":      model,
        "max_tokens": 8096,
        "stream":     true,
        "messages":   api_messages
    });

    let response = match reqwest::Client::new()
        .post("https://api.anthropic.com/v1/messages")
        .headers(headers)
        .json(&body)
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => {
            tx.send(chunk(format!("[api error] {e}"), true)).ok();
            tx.send(state_msg(AgentState::Error)).ok();
            return;
        }
    };

    if !response.status().is_success() {
        let status = response.status();
        let body   = response.text().await.unwrap_or_default();
        tx.send(chunk(format!("[api {status}] {body}"), true)).ok();
        tx.send(state_msg(AgentState::Error)).ok();
        return;
    }

    let mut stream    = response.bytes_stream();
    let mut cancel_rx = std::pin::pin!(cancel_rx);
    let mut buf       = String::new();

    'outer: loop {
        tokio::select! {
            biased;
            _ = &mut cancel_rx => {
                tx.send(state_msg(AgentState::Idle)).ok();
                return;
            }
            bytes = stream.next() => {
                match bytes {
                    None => break,
                    Some(Err(e)) => {
                        tx.send(chunk(format!("[stream error] {e}"), true)).ok();
                        break;
                    }
                    Some(Ok(b)) => {
                        buf.push_str(&String::from_utf8_lossy(&b));
                        // Process all complete lines
                        while let Some(nl) = buf.find('\n') {
                            let line = buf[..nl].trim_end_matches('\r').to_string();
                            buf = buf[nl + 1..].to_string();

                            let Some(data) = line.strip_prefix("data: ") else { continue };
                            if data == "[DONE]" { break 'outer; }

                            if let Ok(v) = serde_json::from_str::<Value>(data) {
                                match v["type"].as_str() {
                                    Some("content_block_delta") => {
                                        if let Some(text) = v["delta"]["text"].as_str() {
                                            send_text_delta(text, &tx, &project_name, &tab_kind);
                                        }
                                    }
                                    Some("message_stop") => break 'outer,
                                    _ => {}
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    tx.send(state_msg(AgentState::Idle)).ok();
}

/// Split a raw text delta on newlines and send as Chunk messages.
fn send_text_delta(
    text:         &str,
    tx:           &mpsc::UnboundedSender<AgentMessage>,
    project_name: &str,
    tab_kind:     &AgentKind,
) {
    let parts: Vec<&str> = text.split('\n').collect();
    let last = parts.len().saturating_sub(1);
    for (i, part) in parts.iter().enumerate() {
        tx.send(AgentMessage::Chunk {
            project_name: project_name.to_string(),
            tab_kind:     tab_kind.clone(),
            text:         part.to_string(),
            is_newline:   i < last,
        }).ok();
    }
}
