use futures_util::stream::SplitStream;
use futures_util::{SinkExt, StreamExt};
use log::{error, info, warn};
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc::{self, UnboundedSender};
use tokio_stream::wrappers::UnboundedReceiverStream;
use tokio_tungstenite::WebSocketStream;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use wasm_peers_protocol::one_to_many::SignalMessage;
use wasm_peers_protocol::{SessionId, UserId};
use webrtc::api::interceptor_registry::register_default_interceptors;
use webrtc::api::media_engine::MediaEngine;
use webrtc::api::APIBuilder;
use webrtc::data_channel::data_channel_state::RTCDataChannelState;
use webrtc::data_channel::RTCDataChannel;
use webrtc::ice_transport::ice_candidate::RTCIceCandidateInit;
use webrtc::ice_transport::ice_server::RTCIceServer;
use webrtc::interceptor::registry::Registry;
use webrtc::peer_connection::configuration::RTCConfiguration;
use webrtc::peer_connection::peer_connection_state::RTCPeerConnectionState;
use webrtc::peer_connection::sdp::session_description::RTCSessionDescription;
use webrtc::peer_connection::signaling_state::RTCSignalingState;
use webrtc::peer_connection::RTCPeerConnection;

#[derive(Debug, Deserialize)]
pub struct IceCandidate {
    pub candidate: String,
    #[serde(rename = "sdpMid")]
    pub sdp_mid: Option<String>,
    #[serde(rename = "sdpMLineIndex")]
    pub sdp_mline_index: Option<u16>,
    #[serde(rename = "usernameFragment")]
    pub username_fragment: Option<String>,
}

pub struct EzRTCHost {
    pub peer_connections: Arc<Mutex<HashMap<UserId, Arc<RTCPeerConnection>>>>,
    pub data_channels: Arc<Mutex<HashMap<UserId, Arc<RTCDataChannel>>>>,
    pub ws_writer: Arc<UnboundedSender<Message>>,
    pub ws_reader: SplitStream<WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>>,
    pub ice_servers: Vec<RTCIceServer>,
}

impl EzRTCHost {
    pub async fn new(session_id: SessionId, host_url: String, ice_servers: Vec<RTCIceServer>) -> Self {
        let global_peer_connections = Arc::new(Mutex::new(HashMap::new()));
        let global_data_channels = Arc::new(Mutex::new(HashMap::new()));

        let is_host: bool = true;
        let session_id: SessionId = session_id;
        let signaling_url = url::Url::parse(&host_url).unwrap();

        // Connect to the signaling server
        let (ws_stream, _response) = connect_async(signaling_url).await.expect("Failed to connect");
        println!("WebSocket handshake has been successfully completed");

        // Create a channel to send and receive WS messages from the signaling server
        let (mut user_ws_write, ws_reader) = ws_stream.split();
        let (write, read) = mpsc::unbounded_channel();
        let mut read = UnboundedReceiverStream::new(read);

        // Send message to the signaling server when the channel is receiving messages
        tokio::task::spawn(async move {
            while let Some(message) = read.next().await {
                user_ws_write.send(message).await.unwrap();
            }
        });

        let ws_writer = Arc::new(write.clone());

        // Send join message
        let join_message = SignalMessage::SessionJoin(session_id.clone(), is_host);
        ws_writer.send(Message::Text(serde_json::to_string(&join_message).unwrap())).unwrap();

        return Self {
            peer_connections: global_peer_connections,
            data_channels: global_data_channels,
            ws_writer,
            ws_reader,
            ice_servers,
        };
    }

    pub async fn start(self) {
        // Receive messages from the signaling server
        let read_future = self.ws_reader.for_each(|message| async {
            info!("Message received from signaling server: {:?}", message);

            match message {
                Ok(message) => {
                    if let Ok(msg) = message.to_text() {
                        match serde_json::from_str::<SignalMessage>(msg) {
                            Ok(request) => match request {
                                SignalMessage::SessionReady(session_id, user_id) => {
                                    // Setup WebRTC
                                    let mut m = MediaEngine::default();
                                    m.register_default_codecs().unwrap();

                                    let mut registry = Registry::new();
                                    registry = register_default_interceptors(registry, &mut m).unwrap();

                                    let api = APIBuilder::new().with_media_engine(m).with_interceptor_registry(registry).build();

                                    let config = RTCConfiguration {
                                        ice_servers: self.ice_servers.clone(),
                                        ..Default::default()
                                    };

                                    let peer_connection = Arc::new(api.new_peer_connection(config).await.unwrap());
                                    let data_channel = peer_connection.create_data_channel("ezrtc-dc", None).await.unwrap();

                                    let offer = peer_connection.create_offer(None).await.unwrap();

                                    peer_connection.set_local_description(offer.clone()).await.unwrap();

                                    self.peer_connections.lock().unwrap().insert(user_id, peer_connection);
                                    self.data_channels.lock().unwrap().insert(user_id, data_channel);

                                    self.ws_writer
                                        .send(Message::Text(serde_json::to_string(&SignalMessage::SdpOffer(session_id, user_id, offer.clone().sdp)).unwrap()))
                                        .unwrap();
                                }
                                SignalMessage::SdpAnswer(_session_id, user_id, sdp_answer) => {
                                    let peer_connection = {
                                        let peer_connections = self.peer_connections.lock().unwrap();
                                        peer_connections.get(&user_id).unwrap().clone()
                                    };
                                    let data_channel = {
                                        let data_channels = self.data_channels.lock().unwrap();
                                        data_channels.get(&user_id).unwrap().clone()
                                    };

                                    let answer = RTCSessionDescription::answer(sdp_answer.clone()).unwrap();

                                    let pc = Arc::clone(&peer_connection);
                                    if peer_connection.signaling_state() == RTCSignalingState::HaveLocalOffer {
                                        pc.set_remote_description(answer).await.unwrap();

                                        warn!("Remote description set");
                                    } else {
                                        return;
                                    }

                                    let dc = Arc::clone(&data_channel);
                                    let pc = Arc::clone(&peer_connection);
                                    let dcs = Arc::clone(&self.data_channels);
                                    let pcs = Arc::clone(&self.peer_connections);
                                    peer_connection.on_peer_connection_state_change(Box::new(move |state| {
                                        warn!("State changed => {:?}", state);

                                        let dc2 = Arc::clone(&dc);
                                        let pc2 = Arc::clone(&pc);
                                        let dcs = Arc::clone(&dcs);
                                        let pcs = Arc::clone(&pcs);
                                        match state {
                                            RTCPeerConnectionState::Disconnected => {
                                                tokio::spawn(async move {
                                                    pc2.close().await.unwrap();
                                                    dc2.close().await.unwrap();

                                                    let mut data_channels = dcs.lock().unwrap();
                                                    let mut peer_connections = pcs.lock().unwrap();

                                                    // Collect keys to remove
                                                    let keys_to_remove: Vec<UserId> =
                                                        data_channels.iter().filter(|(_, v)| v.ready_state() == RTCDataChannelState::Closed).map(|(k, _)| k.clone()).collect();

                                                    // Remove data channels
                                                    for k in keys_to_remove {
                                                        data_channels.remove(&k);
                                                        peer_connections.remove(&k);
                                                    }
                                                });
                                            }
                                            _ => {}
                                        }

                                        Box::pin(async move {})
                                    }));

                                    data_channel.on_open(Box::new(move || {
                                        info!("Data channel opened");

                                        Box::pin(async move {})
                                    }));

                                    data_channel.on_message(Box::new(move |msg| {
                                        info!("Message received: {:?}", msg);

                                        Box::pin(async move {})
                                    }));
                                }
                                // SignalMessage::SdpOffer(session_id, user_id, sdp_offer) => {}
                                SignalMessage::IceCandidate(session_id, user_id, ice_candidate) => {
                                    let peer_connection = {
                                        let peer_connections = self.peer_connections.lock().unwrap();
                                        peer_connections.get(&user_id).unwrap().clone()
                                    };

                                    info!("Ice candidate received: {:?}, {session_id}, {user_id}", ice_candidate);

                                    let candidate = serde_json::from_str::<IceCandidate>(ice_candidate.as_str()).unwrap();

                                    let candidate_init = RTCIceCandidateInit {
                                        candidate: candidate.candidate,
                                        sdp_mid: candidate.sdp_mid,
                                        sdp_mline_index: candidate.sdp_mline_index,
                                        username_fragment: candidate.username_fragment,
                                    };

                                    peer_connection.add_ice_candidate(candidate_init).await.unwrap();
                                }
                                _ => {}
                            },
                            Err(error) => {
                                error!("Error parsing message from server: {:?}", error);
                            }
                        }
                    }
                }
                Err(error) => {
                    error!("Error receiving message from server: {:?}", error);
                }
            }
        });

        read_future.await;
    }
}
