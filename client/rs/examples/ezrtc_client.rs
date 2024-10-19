use ezrtc::{client::EzRTCClient, socket::DataChannelHandler};
use log::{info, LevelFilter};
use simplelog::{ColorChoice, TermLogger, TerminalMode};
use std::sync::Arc;
use webrtc::{
    data_channel::{data_channel_state::RTCDataChannelState, RTCDataChannel},
    ice_transport::ice_server::RTCIceServer,
};

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
                // Send a message every 3 seconds to host
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
    }

    // Start the connection
    let _client = EzRTCClient::new(
        "ws://localhost:9001/one-to-many".to_string(), // ezrtc-server address
        "random_session_id".to_string(),
        ice_servers,
        Arc::new(Box::new(MyDataChannelHandler {})),
    )
    .await;

    // Loop forever
    loop {
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    }
}
