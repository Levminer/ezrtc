use std::sync::Arc;

use log::{warn, LevelFilter};
use simplelog::{ColorChoice, Config, TermLogger, TerminalMode};
use wasm_peers_protocol::SessionId;
use webrtc::{
    data_channel::data_channel_state::RTCDataChannelState,
    ice_transport::{ice_credential_type::RTCIceCredentialType, ice_server::RTCIceServer},
};

mod host;

#[tokio::main]
pub async fn main() {
    TermLogger::init(LevelFilter::Info, Config::default(), TerminalMode::Mixed, ColorChoice::Auto).unwrap();

    let server = host::EzRTCHost::new(
        SessionId::new(String::from("crs_696969")),
        String::from("wss://rtc-usw.levminer.com/one-to-many"),
        vec![RTCIceServer {
            urls: vec!["stun:openrelay.metered.ca:80".to_owned(), "turn:standard.relay.metered.ca:443".to_owned()],
            credential: "8By67N7nOLDIagJk".to_owned(),
            username: "2ce7aaf275c1abdef74ec7e3".to_owned(),
            credential_type: RTCIceCredentialType::Password,
        }],
    )
    .await;

    let peers = Arc::clone(&server.peer_connections);
    let data = Arc::clone(&server.data_channels);
    tokio::spawn(async {
        server.start().await;
    });

    loop {
        for (user_id, data) in data.lock().unwrap().iter() {
            if data.ready_state() == RTCDataChannelState::Open {
                data.send_text("test".to_string()).await.unwrap();
            }
        }

        warn!("dc length {}", data.lock().unwrap().len());

        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }
}
