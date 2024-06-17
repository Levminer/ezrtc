# ezrtc

-   Easy cross-platform WebRTC communication with data channels and a simple signaling server.

## Usage

```rs
use ezrtc::{host::EzRTCHost, socket::DataChannelHandler};
use log::{info, warn, LevelFilter};
use simplelog::{ColorChoice, TermLogger, TerminalMode};
use std::sync::Arc;
use webrtc::ice_transport::ice_server::RTCIceServer;

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
        fn handle_data_channel_open(&self) {
            warn!("Data channel opened!");
        }

        fn handle_data_channel_message(&self, message: String) {
            warn!("Data channel message received: {:?}", message);
        }
    }

    // Start the connection
    let host = EzRTCHost::new(
        "wss://your-signaling-server.com/one-to-many".to_string(),
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
```
