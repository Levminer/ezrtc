use crate::protocol::SessionId;
use crate::socket::{DataChannelHandler, WSClient};
use ezsockets::{ClientConfig, SocketConfig};
use log::info;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use webrtc::api::interceptor_registry::register_default_interceptors;
use webrtc::api::media_engine::MediaEngine;
use webrtc::api::APIBuilder;
use webrtc::data_channel::data_channel_message::DataChannelMessage;
use webrtc::data_channel::RTCDataChannel;
use webrtc::ice_transport::ice_server::RTCIceServer;
use webrtc::interceptor::registry::Registry;
use webrtc::peer_connection::configuration::RTCConfiguration;
use webrtc::peer_connection::RTCPeerConnection;

pub struct EzRTCClient {
    pub peer_connection: Arc<Mutex<Arc<RTCPeerConnection>>>,
    pub ice_servers: Vec<RTCIceServer>,
    pub handle: ezsockets::Client<WSClient>,
}

impl EzRTCClient {
    pub async fn new(host_url: String, session_id: String, ice_servers: Vec<RTCIceServer>, data_channel_handler: Arc<Box<dyn DataChannelHandler>>) -> Self {
        // Setup WebRTC
        let mut m = MediaEngine::default();
        m.register_default_codecs().unwrap();

        let mut registry = Registry::new();
        registry = register_default_interceptors(registry, &mut m).unwrap();

        let api = APIBuilder::new().with_media_engine(m).with_interceptor_registry(registry).build();

        let config = RTCConfiguration {
            ice_servers: ice_servers.clone(),
            ..Default::default()
        };

        let peer_connection = Arc::new(api.new_peer_connection(config).await.unwrap());

        let pc = peer_connection.clone();
        let dh = data_channel_handler.clone();
        pc.on_data_channel(Box::new(move |d: Arc<RTCDataChannel>| {
            let dc = Arc::clone(&d);
            let dh2 = dh.clone();

            // Register channel opening handling
            Box::pin(async move {
                // Handle the channel opening
                let dh3 = dh2.clone();
                d.on_open(Box::new(move || {
                    let dc2 = Arc::clone(&dc);
                    dh3.handle_data_channel_open(dc2);

                    Box::pin(async move {})
                }));

                // Handle the received messages
                let dh3 = dh2.clone();
                d.on_message(Box::new(move |msg: DataChannelMessage| {
                    //Convert message to string
                    let message = String::from_utf8(msg.data.to_vec()).unwrap();
                    dh3.handle_data_channel_message(message);

                    Box::pin(async move {})
                }));

                // Handle the channel closing
                d.on_close(Box::new(move || {
                    info!("Data channel closed");
                    Box::pin(async {})
                }));
            })
        }));

        let global_peer_connection = Arc::new(Mutex::new(peer_connection));
        let signaling_url = url::Url::parse(&host_url).expect("Invalid host URL");

        let pc = Arc::clone(&global_peer_connection);
        let ice = ice_servers.clone();

        let config = ClientConfig::new(signaling_url);
        let config = config.socket_config(SocketConfig {
            heartbeat: Duration::from_secs(60),
            timeout: Duration::from_secs(90),
            ..SocketConfig::default()
        });

        let (handle, future) = ezsockets::connect(
            |handle| WSClient {
                handle,
                session_id: SessionId::new(session_id),
                peer_connection: pc,
                ice_servers: ice,
                data_channel_handler,
            },
            config,
        )
        .await;

        tokio::spawn(async move {
            info!("Connected to signaling server");
            future.await.unwrap();
        });

        return Self {
            peer_connection: global_peer_connection.clone(),
            ice_servers: ice_servers.clone(),
            handle,
        };
    }
}
