use axum::extract::{Path, State, WebSocketUpgrade};
use axum::response::Response;
use axum::routing::get;
use axum::{Json, Router};
use ezrtc::protocol::SessionId;
use serde::{Deserialize, Serialize};

use crate::one_to_many;

#[derive(Default, Clone)]
pub struct ServerState {
    one_to_many_connections: one_to_many::Connections,
    one_to_many_sessions: one_to_many::Sessions,
    one_to_many_pings: one_to_many::Pings,
}

#[derive(Serialize, Deserialize)]
struct RootMessage {
    status: u8,
    build: String,
}

#[derive(Serialize, Deserialize)]
struct StatusMessage {
    online: bool,
}

const VERSION: &str = env!("CARGO_PKG_VERSION");

async fn root() -> Json<RootMessage> {
    Json(RootMessage {
        status: 200,
        build: VERSION.to_string(),
    })
}

#[allow(clippy::unused_async)]
async fn health_handler() -> &'static str {
    "OK"
}

#[allow(clippy::unused_async)]
async fn one_to_many_handler(State(state): State<ServerState>, ws: WebSocketUpgrade) -> Response {
    ws.on_upgrade(move |socket| one_to_many::user_connected(socket, state.one_to_many_connections, state.one_to_many_sessions, state.one_to_many_pings))
}

async fn status_handler(Path(session_id): Path<String>, State(state): State<ServerState>) -> Json<StatusMessage> {
    let pings = state.one_to_many_pings.lock().unwrap().clone();

    // iterate over all pings and return that matches the session_id from path
    let ping = pings
        .iter()
        .find_map(|(_k, v)| if v.session_id == Some(SessionId::new(session_id.clone())) { Some(v.clone()) } else { None });

    match ping {
        Some(ping) => Json(StatusMessage { online: ping.online }),
        None => Json(StatusMessage { online: false }),
    }
}

pub fn create(server_state: ServerState) -> Router {
    Router::new()
        .route("/health", get(health_handler))
        .route("/", get(root))
        .route("/one-to-many", get(one_to_many_handler))
        .route("/status/:id", get(status_handler))
        .with_state(server_state)
}
