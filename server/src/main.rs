use std::env;
use std::net::SocketAddr;
use std::str::FromStr;

use ezrtc_server::router::{self, ServerState};
use log::LevelFilter;
use simplelog::{ColorChoice, Config, TermLogger, TerminalMode};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    TermLogger::init(LevelFilter::Info, Config::default(), TerminalMode::Mixed, ColorChoice::Auto)?;

    let server_state = ServerState::default();
    let app = router::create(server_state);

    let address = env::args().nth(1).unwrap_or_else(|| "0.0.0.0:9001".to_string());
    let socket_addr = SocketAddr::from_str(&address)?;
    let listener = tokio::net::TcpListener::bind(socket_addr).await?;

    axum::serve::serve(listener, app.into_make_service()).await?;

    Ok(())
}
