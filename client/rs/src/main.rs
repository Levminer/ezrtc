use futures_util::{SinkExt, StreamExt};
use log::LevelFilter;
use log::{error, info, warn};
use simplelog::{ColorChoice, Config, TermLogger, TerminalMode};
use std::sync::{Arc, Mutex};
use tokio::io::Result;
use tokio::sync::mpsc;
use tokio_stream::wrappers::UnboundedReceiverStream;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use wasm_peers_protocol::one_to_many::SignalMessage;
use wasm_peers_protocol::SessionId;
use webrtc::api::interceptor_registry::register_default_interceptors;
use webrtc::api::media_engine::MediaEngine;
use webrtc::api::APIBuilder;
use webrtc::data_channel::data_channel_message::DataChannelMessage;
use webrtc::data_channel::RTCDataChannel;
use webrtc::ice_transport::ice_candidate::RTCIceCandidate;
use webrtc::ice_transport::ice_credential_type::RTCIceCredentialType;
use webrtc::ice_transport::ice_server::RTCIceServer;
use webrtc::interceptor::registry::Registry;
use webrtc::peer_connection::configuration::RTCConfiguration;
use webrtc::peer_connection::sdp::session_description::RTCSessionDescription;

#[tokio::main]
pub async fn main() -> Result<()> {
    TermLogger::init(
        LevelFilter::Info,
        Config::default(),
        TerminalMode::Mixed,
        ColorChoice::Auto,
    )
    .unwrap();

    let is_host: bool = false;
    let session_id: SessionId = SessionId::new(String::from("crs_1d7de676b4"));
    let host_url = "wss://slippery-chalk-production.up.railway.app/one-to-many";
    let url = url::Url::parse(host_url).unwrap();

    // Connect to the websocket server
    let (ws_stream, _response) = connect_async(url).await.expect("Failed to connect");
    println!("WebSocket handshake has been successfully completed");

    // Create a channel to send and receive WS messages
    let (mut user_ws_write, user_ws_read) = ws_stream.split();
    let (write, read) = mpsc::unbounded_channel();
    let mut read = UnboundedReceiverStream::new(read);

    // Send message when the channel is receiving messages
    tokio::task::spawn(async move {
        while let Some(message) = read.next().await {
            user_ws_write.send(message).await.unwrap();
        }
    });

    // Setup WebRTC
    let mut m = MediaEngine::default();
    m.register_default_codecs().unwrap();

    let mut registry = Registry::new();
    registry = register_default_interceptors(registry, &mut m).unwrap();

    let api = APIBuilder::new()
        .with_media_engine(m)
        .with_interceptor_registry(registry)
        .build();

    let config = RTCConfiguration {
        ice_servers: vec![RTCIceServer {
            urls: vec![
                "stun:openrelay.metered.ca:80".to_owned(),
                "turn:standard.relay.metered.ca:443".to_owned(),
            ],
            credential: "8By67N7nOLDIagJk".to_owned(),
            username: "2ce7aaf275c1abdef74ec7e3".to_owned(),
            credential_type: RTCIceCredentialType::Password,
        }],

        ..Default::default()
    };

    let peer_connection = Arc::new(api.new_peer_connection(config).await.unwrap());
    let writer = Arc::new(write.clone());
    let send = Arc::new(Mutex::new(true));

    // Setup  WebRTC DataChannel
    peer_connection.on_data_channel(Box::new(move |d: Arc<RTCDataChannel>| {
        let d_label = d.label().to_owned();
        let d_id = d.id();
        println!("New DataChannel {d_label} {d_id}");

        // Register channel opening handling
        Box::pin(async move {
            let d_label2 = d_label.clone();
            let d_id2 = d_id;

            // Handle the channel opening
            d.on_open(Box::new(move || {
                info!("Data channel '{d_label2}'-'{d_id2}' open.");
                Box::pin(async {})
            }));

            // Handle the received messages
            d.on_message(Box::new(move |msg: DataChannelMessage| {
                let msg_str = String::from_utf8(msg.data.to_vec()).unwrap();
                info!("Message from DataChannel '{d_label}': '{msg_str}'");
                Box::pin(async {})
            }));

            // Handle the channel closing
            d.on_close(Box::new(move || {
                info!("Data channel closed");
                Box::pin(async {})
            }));
        })
    }));

    // Send join message
    let join_message = SignalMessage::SessionJoin(session_id, is_host);
    writer
        .send(Message::Text(serde_json::to_string(&join_message).unwrap()))
        .unwrap();

    // Receive messages from the server
    let read_future = user_ws_read.for_each(|message| async {
        info!("Message received from server: {:?}", message);

        match message {
            Ok(message) => {
                if let Ok(msg) = message.to_text() {
                    match serde_json::from_str::<SignalMessage>(msg) {
                        Ok(request) => match request {
                            SignalMessage::SessionReady(session_id, user_id) => {}
                            SignalMessage::SdpAnswer(session_id, user_id, sdp_answer) => {}
                            SignalMessage::SdpOffer(session_id, user_id, sdp_offer) => {
                                let offer = RTCSessionDescription::offer(sdp_offer).unwrap();

                                peer_connection.set_remote_description(offer).await.unwrap();

                                let answer = peer_connection.create_answer(None).await.unwrap();

                                peer_connection
                                    .set_local_description(answer.clone())
                                    .await
                                    .unwrap();

                                let pc = Arc::downgrade(&peer_connection);
                                let wr = Arc::downgrade(&writer);
                                let send = Arc::downgrade(&send);

                                peer_connection.on_ice_candidate(Box::new(
                                    move |candidate: Option<RTCIceCandidate>| {
                                        info!("on_ice_candidate {:?}", candidate);

                                        let pc2 = pc.clone();
                                        let wr2 = wr.clone();
                                        let session_id2 = session_id.clone();
                                        let send2 = send.clone();

                                        Box::pin(async move {
                                            if let Some(c) = candidate {
                                                info!("Ice candidate received: {:?}", c);

                                                if let Some(pc) = pc2.upgrade() {
                                                    let ld = pc.local_description().await.unwrap();

                                                    if let Some(wr) = wr2.upgrade() {
                                                        if let Some(send) = send2.upgrade() {
                                                            let mut val = send.lock().unwrap();

                                                            // Make sure only one answer is sent
                                                            if *val {
                                                                info!("sending answer");

                                                                wr.send(Message::Text(
                                                                    serde_json::to_string(
                                                                        &SignalMessage::SdpAnswer(
                                                                            session_id2,
                                                                            user_id,
                                                                            ld.sdp,
                                                                        ),
                                                                    )
                                                                    .unwrap(),
                                                                ))
                                                                .unwrap();

                                                                *val = false;
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        })
                                    },
                                ));
                            }
                            SignalMessage::IceCandidate(session_id, user_id, ice_candidate) => {
                                info!(
                                    "Ice candidate received: {:?}, {session_id}, {user_id}",
                                    ice_candidate
                                );
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

    Ok(())
}
