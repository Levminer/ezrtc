use ezrtc::{host::EzRTCHost, protocol::{SignalMessage, Status, UserId}, socket::{DataChannelHandler, WSHost}, RTCDataChannel, RTCDataChannelState, RTCIceServer};
use log::{info, LevelFilter};
use simplelog::{ColorChoice, TermLogger, TerminalMode};
use std::sync::Arc;

#[tokio::main]
pub async fn main() {
    TermLogger::init(LevelFilter::Info, Default::default(), TerminalMode::Mixed, ColorChoice::Auto).unwrap();

    // Define your STUN and TURN servers here
    let ice_servers = vec![RTCIceServer {
        urls: vec!["stun:stun.cloudflare.com:3478".to_owned()],
        ..Default::default()
    }];

    // Define your data channel handler
    struct MyDataChannelHandler {}

    impl DataChannelHandler for MyDataChannelHandler {
        fn handle_data_channel_open(&self, dc: Arc<RTCDataChannel>) {
            info!("Data channel opened!");

            tokio::spawn(async move {
                // Send a message every 3 seconds to connected clients
                loop {
                    if dc.ready_state() == RTCDataChannelState::Open {
                        dc.send_text("test".to_string()).await.unwrap();
                    }

                    tokio::time::sleep(std::time::Duration::from_secs(3)).await;
                }
            });
        }

        fn handle_data_channel_message(&self, message: String) {
            info!("Data channel message received: {:?}", message);
        }

        fn handle_keep_alive(&self, handle: &mut WSHost, user_id: UserId) {
            let ping_message = SignalMessage::KeepAlive(
                user_id,
                Status {
                    session_id: Some(handle.session_id.clone()),
                    is_host: Some(true),
                    version: Some(env!("CARGO_PKG_VERSION").to_string()),
                    metadata: Some(serde_json::json!({"test": "test",})),
                },
            );
            handle.handle.text(serde_json::to_string(&ping_message).unwrap()).unwrap();

            info!("Sending pong to server");
        }
    }

    // Start the connection
    let host = EzRTCHost::new(
        "ws://localhost:9001/one-to-many".to_string(), // ezrtc-server address
        "random_session_id".to_string(),
        ice_servers,
        Arc::new(Box::new(MyDataChannelHandler {})),
    )
    .await;

    // Log connected clients number every 5 seconds
    loop {
        info!("Connected clients: {:?}", host.peer_connections.lock().unwrap().len());

        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    }
}
