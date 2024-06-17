use crate::protocol::{SessionId, UserId};
use crate::socket::{DataChannelHandler, WSClient};
use ezsockets::ClientConfig;
use log::info;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use webrtc::data_channel::RTCDataChannel;
use webrtc::ice_transport::ice_server::RTCIceServer;
use webrtc::peer_connection::RTCPeerConnection;

pub struct EzRTCHost {
    pub peer_connections: Arc<Mutex<HashMap<UserId, Arc<RTCPeerConnection>>>>,
    pub data_channels: Arc<Mutex<HashMap<UserId, Arc<RTCDataChannel>>>>,
    pub ice_servers: Vec<RTCIceServer>,
    pub handle: ezsockets::Client<WSClient>,
}

impl EzRTCHost {
    pub async fn new(host_url: String, session_id: String, ice_servers: Vec<RTCIceServer>, data_channel_handler: Arc<Box<dyn DataChannelHandler>>) -> Self {
        let global_peer_connections = Arc::new(Mutex::new(HashMap::new()));
        let global_data_channels = Arc::new(Mutex::new(HashMap::new()));
        let signaling_url = url::Url::parse(&host_url).expect("Invalid host URL");

        let dc = Arc::clone(&global_data_channels);
        let pc = Arc::clone(&global_peer_connections);
        let ice = ice_servers.clone();

        let config = ClientConfig::new(signaling_url);
        let (handle, future) = ezsockets::connect(
            |handle| WSClient {
                handle,
                session_id: SessionId::new(session_id),
                data_channels: dc,
                peer_connections: pc,
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
            peer_connections: global_peer_connections.clone(),
            data_channels: global_data_channels.clone(),
            ice_servers: ice_servers.clone(),
            handle,
        };
    }
}
