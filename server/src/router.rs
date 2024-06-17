use axum::extract::{State, WebSocketUpgrade};
use axum::response::Response;
use axum::routing::get;
use axum::{Json, Router};
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
    ws.on_upgrade(move |socket| {
        one_to_many::user_connected(
            socket,
            state.one_to_many_connections,
            state.one_to_many_sessions,
            state.one_to_many_pings,
        )
    })
}

pub fn create(server_state: ServerState) -> Router {
    Router::new()
        .route("/health", get(health_handler))
        .route("/", get(root))
        .route("/one-to-many", get(one_to_many_handler))
        .with_state(server_state)
}
