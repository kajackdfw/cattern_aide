use std::sync::Arc;
use axum::{body::Body, extract::State, http::Request, response::Response, Router};
use reqwest::Client;
use tokio::sync::{mpsc, oneshot};
use crate::agent::{AgentKind, AgentMessage, AgentState};

struct ProxyState {
    client:       Client,
    target:       String,
    tx:           mpsc::UnboundedSender<AgentMessage>,
    project_name: String,
    tab_kind:     AgentKind,
}

async fn proxy_handler(
    State(state): State<Arc<ProxyState>>,
    req: Request<Body>,
) -> Response {
    let method = req.method().clone();
    let uri    = req.uri().clone();
    let path_and_query = uri.path_and_query()
        .map(|pq| pq.to_string())
        .unwrap_or_else(|| "/".to_string());

    let target_url = format!("{}{}", state.target, path_and_query);

    let _ = state.tx.send(AgentMessage::Chunk {
        project_name: state.project_name.clone(),
        tab_kind:     state.tab_kind.clone(),
        text:         format!("→ {} {}", method, path_and_query),
        is_newline:   true,
    });

    let mut req_builder = state.client.request(method, &target_url);
    for (name, value) in req.headers() {
        let n = name.as_str();
        if !matches!(n, "host" | "connection" | "transfer-encoding" | "upgrade") {
            req_builder = req_builder.header(name, value);
        }
    }

    let body_bytes = axum::body::to_bytes(req.into_body(), 4 * 1024 * 1024)
        .await
        .unwrap_or_default();
    if !body_bytes.is_empty() {
        req_builder = req_builder.body(body_bytes.to_vec());
    }

    match req_builder.send().await {
        Ok(resp) => {
            let status = resp.status();
            let _ = state.tx.send(AgentMessage::Chunk {
                project_name: state.project_name.clone(),
                tab_kind:     state.tab_kind.clone(),
                text:         format!("← {} {}", status.as_u16(), path_and_query),
                is_newline:   true,
            });

            let mut builder = axum::http::Response::builder().status(status.as_u16());
            for (name, value) in resp.headers() {
                let n = name.as_str();
                if !matches!(n, "connection" | "transfer-encoding") {
                    builder = builder.header(name, value);
                }
            }
            let resp_bytes = resp.bytes().await.unwrap_or_default();
            builder.body(Body::from(resp_bytes)).unwrap_or_else(|_| {
                axum::http::Response::builder()
                    .status(500)
                    .body(Body::empty())
                    .unwrap()
            })
        }
        Err(e) => {
            let _ = state.tx.send(AgentMessage::Chunk {
                project_name: state.project_name.clone(),
                tab_kind:     state.tab_kind.clone(),
                text:         format!("✗ {} → {}", path_and_query, e),
                is_newline:   true,
            });
            axum::http::Response::builder()
                .status(502)
                .body(Body::from(format!("Proxy error: {e}")))
                .unwrap()
        }
    }
}

pub async fn run(
    project_name: String,
    port:         u16,
    target:       String,
    tab_kind:     AgentKind,
    tx:           mpsc::UnboundedSender<AgentMessage>,
    cancel_rx:    oneshot::Receiver<()>,
) {
    let _ = tx.send(AgentMessage::StateChange {
        project_name: project_name.clone(),
        tab_kind:     tab_kind.clone(),
        new_state:    AgentState::Running,
    });

    let state = Arc::new(ProxyState {
        client:       Client::new(),
        target:       target.trim_end_matches('/').to_string(),
        tx:           tx.clone(),
        project_name: project_name.clone(),
        tab_kind:     tab_kind.clone(),
    });

    let app = Router::new()
        .fallback(proxy_handler)
        .with_state(state);

    let bind_addr = format!("127.0.0.1:{port}");
    let target_display = target.clone();

    match tokio::net::TcpListener::bind(&bind_addr).await {
        Ok(listener) => {
            let _ = tx.send(AgentMessage::Chunk {
                project_name: project_name.clone(),
                tab_kind:     tab_kind.clone(),
                text:         format!("Listening on {bind_addr} → {target_display}"),
                is_newline:   true,
            });
            let _ = axum::serve(listener, app)
                .with_graceful_shutdown(async move { cancel_rx.await.ok(); })
                .await;
        }
        Err(e) => {
            let _ = tx.send(AgentMessage::Chunk {
                project_name: project_name.clone(),
                tab_kind:     tab_kind.clone(),
                text:         format!("Failed to bind {bind_addr}: {e}"),
                is_newline:   true,
            });
        }
    }

    let _ = tx.send(AgentMessage::StateChange {
        project_name,
        tab_kind,
        new_state: AgentState::Idle,
    });
}
