use crate::protocol::{IceCandidateJSON, SessionId, SignalMessage, UserId};
use async_trait::async_trait;
use ezsockets::client::ClientCloseMode;
use ezsockets::CloseFrame;
use ezsockets::Error;
use ezsockets::WSError;
use log::{error, info, warn};
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;
use webrtc::api::interceptor_registry::register_default_interceptors;
use webrtc::api::media_engine::MediaEngine;
use webrtc::api::APIBuilder;
use webrtc::data_channel::data_channel_state::RTCDataChannelState;
use webrtc::data_channel::RTCDataChannel;
use webrtc::ice_transport::ice_candidate::RTCIceCandidate;
use webrtc::ice_transport::ice_candidate::RTCIceCandidateInit;
use webrtc::ice_transport::ice_server::RTCIceServer;
use webrtc::interceptor::registry::Registry;
use webrtc::peer_connection::configuration::RTCConfiguration;
use webrtc::peer_connection::peer_connection_state::RTCPeerConnectionState;
use webrtc::peer_connection::sdp::session_description::RTCSessionDescription;
use webrtc::peer_connection::signaling_state::RTCSignalingState;
use webrtc::peer_connection::RTCPeerConnection;

pub enum WSCall {}

pub struct WSHost {
    pub session_id: SessionId,
    pub peer_connections: Arc<Mutex<HashMap<UserId, Arc<RTCPeerConnection>>>>,
    pub data_channels: Arc<Mutex<HashMap<UserId, Arc<RTCDataChannel>>>>,
    pub ice_servers: Vec<RTCIceServer>,
    pub handle: ezsockets::Client<Self>,
    pub data_channel_handler: Arc<Box<dyn DataChannelHandler>>,
}

pub struct WSClient {
    pub session_id: SessionId,
    pub peer_connection: Arc<Mutex<Arc<RTCPeerConnection>>>,
    pub ice_servers: Vec<RTCIceServer>,
    pub handle: ezsockets::Client<Self>,
    pub data_channel_handler: Arc<Box<dyn DataChannelHandler>>,
}

pub trait DataChannelHandler: Send + Sync {
    fn handle_data_channel_open(&self, dc: Arc<RTCDataChannel>);
    fn handle_data_channel_message(&self, message: String);
    fn handle_keep_alive(&self, handle: &mut WSHost, user_id: UserId);
}

#[async_trait]
impl ezsockets::ClientExt for WSHost {
    type Call = WSCall;

    async fn on_text(&mut self, text: String) -> Result<(), Error> {
        info!("Message received from signaling server: {:?}", text);

        match serde_json::from_str::<SignalMessage>(&text) {
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

                    self.handle
                        .text(serde_json::to_string(&SignalMessage::SdpOffer(session_id, user_id, offer.clone().sdp)).unwrap())
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
                                // TODO Check for other states, web disconnects don't always register
                                RTCPeerConnectionState::Disconnected => {
                                    tokio::spawn(async move {
                                        pc2.close().await.unwrap();
                                        dc2.close().await.unwrap();

                                        let mut data_channels = dcs.lock().unwrap();
                                        let mut peer_connections = pcs.lock().unwrap();

                                        // Collect keys to remove
                                        let keys_to_remove: Vec<UserId> = data_channels.iter().filter(|(_, v)| v.ready_state() == RTCDataChannelState::Closed).map(|(k, _)| k.clone()).collect();

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

                        let dc_handler = self.data_channel_handler.clone();
                        let dc = Arc::clone(&data_channel);
                        data_channel.on_open(Box::new(move || {
                            let dc2 = Arc::clone(&dc);
                            dc_handler.handle_data_channel_open(dc2);

                            Box::pin(async move {})
                        }));

                        let dc_handler = self.data_channel_handler.clone();
                        data_channel.on_message(Box::new(move |msg| {
                            // Convert message to string
                            let message = String::from_utf8(msg.data.to_vec()).unwrap();
                            dc_handler.handle_data_channel_message(message);

                            Box::pin(async move {})
                        }));
                    }
                }
                // SignalMessage::SdpOffer(session_id, user_id, sdp_offer) => {}
                SignalMessage::IceCandidate(_session_id, user_id, ice_candidate) => {
                    let peer_connection = {
                        let peer_connections = self.peer_connections.lock().unwrap();
                        peer_connections.get(&user_id).unwrap().clone()
                    };

                    let candidate = serde_json::from_str::<IceCandidateJSON>(ice_candidate.as_str()).unwrap();

                    let candidate_init = RTCIceCandidateInit {
                        candidate: candidate.candidate,
                        sdp_mid: candidate.sdp_mid,
                        sdp_mline_index: candidate.sdp_mline_index,
                        username_fragment: candidate.username_fragment,
                    };

                    peer_connection.add_ice_candidate(candidate_init).await.unwrap();
                }
                SignalMessage::KeepAlive(user_id, _status) => {
                    let dc_handler = self.data_channel_handler.clone();

                    dc_handler.handle_keep_alive(self, user_id);
                }
                _ => {}
            },
            Err(error) => {
                error!("Error parsing message from server: {:?}", error);
            }
        }

        Ok(())
    }

    async fn on_binary(&mut self, bytes: Vec<u8>) -> Result<(), Error> {
        info!("received bytes: {bytes:?}");
        Ok(())
    }

    async fn on_connect(&mut self) -> Result<(), Error> {
        info!("Connected to server");
        let join_message = SignalMessage::SessionJoin(self.session_id.clone(), true);

        self.handle.text(serde_json::to_string(&join_message).unwrap()).unwrap();
        Ok(())
    }

    async fn on_call(&mut self, _call: Self::Call) -> Result<(), Error> {
        Ok(())
    }

    async fn on_connect_fail(&mut self, _error: WSError) -> Result<ClientCloseMode, Error> {
        error!("Connection failed");
        Ok(ClientCloseMode::Reconnect)
    }

    async fn on_close(&mut self, frame: Option<CloseFrame>) -> Result<ClientCloseMode, Error> {
        error!("Connection closed: {:?}", frame);
        Ok(ClientCloseMode::Reconnect)
    }

    async fn on_disconnect(&mut self) -> Result<ClientCloseMode, Error> {
        error!("Connection disconnected");
        Ok(ClientCloseMode::Reconnect)
    }
}

#[async_trait]
impl ezsockets::ClientExt for WSClient {
    type Call = WSCall;

    async fn on_text(&mut self, text: String) -> Result<(), Error> {
        info!("Message received from signaling server: {:?}", text);

        match serde_json::from_str::<SignalMessage>(&text) {
            Ok(request) => match request {
                SignalMessage::SdpOffer(session_id, user_id, sdp_offer) => {
                    let peer_connection = self.peer_connection.lock().unwrap().clone();

                    let offer = RTCSessionDescription::offer(sdp_offer).unwrap();

                    peer_connection.set_remote_description(offer).await.unwrap();

                    let answer = peer_connection.create_answer(None).await.unwrap();

                    peer_connection.set_local_description(answer.clone()).await.unwrap();

                    info!("Remote description set");

                    let pc = Arc::downgrade(&peer_connection);
                    let hndl = self.handle.clone();

                    peer_connection.on_ice_candidate(Box::new(move |candidate: Option<RTCIceCandidate>| {
                        info!("Ica candidate received: {:?}", candidate);

                        let pc2 = pc.clone();
                        let session_id2 = session_id.clone();
                        let hndl2 = hndl.clone();

                        Box::pin(async move {
                            if let Some(c) = candidate {
                                if let Some(pc) = pc2.upgrade() {
                                    let ld = pc.local_description().await.unwrap();

                                    info!("sending answer {c}");

                                    hndl2.text(serde_json::to_string(&SignalMessage::SdpAnswer(session_id2, user_id, ld.sdp)).unwrap()).unwrap();

                                    // TODO send ice candidate
                                }
                            }
                        })
                    }));
                }
                // SignalMessage::Ping(_is_host, user_id) => {
                //     let ping_message = SignalMessage::Ping(true, user_id);
                //     self.handle.text(serde_json::to_string(&ping_message).unwrap()).unwrap();

                //     info!("Sending pong to server");
                // }
                _ => {}
            },
            Err(error) => {
                error!("Error parsing message from server: {:?}", error);
            }
        }

        Ok(())
    }

    async fn on_binary(&mut self, bytes: Vec<u8>) -> Result<(), Error> {
        info!("received bytes: {bytes:?}");
        Ok(())
    }

    async fn on_connect(&mut self) -> Result<(), Error> {
        info!("Connected to server");
        let join_message = SignalMessage::SessionJoin(self.session_id.clone(), false);

        self.handle.text(serde_json::to_string(&join_message).unwrap()).unwrap();
        Ok(())
    }

    async fn on_call(&mut self, _call: Self::Call) -> Result<(), Error> {
        Ok(())
    }

    async fn on_connect_fail(&mut self, _error: WSError) -> Result<ClientCloseMode, Error> {
        error!("Connection failed");
        Ok(ClientCloseMode::Reconnect)
    }

    async fn on_close(&mut self, frame: Option<CloseFrame>) -> Result<ClientCloseMode, Error> {
        error!("Connection closed: {:?}", frame);
        Ok(ClientCloseMode::Reconnect)
    }

    async fn on_disconnect(&mut self) -> Result<ClientCloseMode, Error> {
        error!("Connection disconnected");
        Ok(ClientCloseMode::Reconnect)
    }
}
