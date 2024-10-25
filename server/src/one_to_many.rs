use anyhow::anyhow;
use axum::extract::ws::{CloseFrame, Message, WebSocket};
use ezrtc::protocol::{SessionId, SignalMessage, UserId};
use futures_util::{SinkExt, StreamExt};
use log::{error, info, warn};
use std::borrow::Cow;
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::{mpsc, RwLock};
use tokio::time;
use tokio_stream::wrappers::UnboundedReceiverStream;

#[derive(Default, Debug)]
pub struct Session {
    pub host: Option<UserId>,
    pub users: HashSet<UserId>,
}

#[derive(Default, Debug)]
pub struct Ping {
    pub online: bool,
    pub session_id: Option<SessionId>,
}

pub type Connections = Arc<RwLock<HashMap<UserId, mpsc::UnboundedSender<Message>>>>;
pub type Sessions = Arc<RwLock<HashMap<SessionId, Session>>>;
pub type Pings = Arc<Mutex<HashMap<UserId, Arc<Ping>>>>;

static NEXT_USER_ID: AtomicUsize = AtomicUsize::new(1);

pub async fn user_connected(ws: WebSocket, connections: Connections, sessions: Sessions, pings: Pings) {
    let user_id = UserId::new(NEXT_USER_ID.fetch_add(1, Ordering::Relaxed));
    info!("new user connected: {:?}", user_id);

    let (mut ws_send, mut ws_recv) = ws.split();

    // Create a channel for sending and receiving ws messages
    let (tx, rx) = mpsc::unbounded_channel();
    let mut rx = UnboundedReceiverStream::new(rx);

    // Ping client every 60 seconds
    let tx2 = tx.clone();
    let user_id2 = user_id.clone();
    let pings2 = pings.clone();

    let mut ping_task = tokio::spawn(async move {
        let mut interval = time::interval(Duration::from_secs(60));

        loop {
            interval.tick().await;

            let status = {
                let pings = pings2.lock().unwrap();
                pings.get(&user_id2).cloned()
            };

            if let Some(ping) = status {
                if ping.online {
                    pings2.lock().unwrap().insert(
                        user_id2.clone(),
                        Arc::new(Ping {
                            online: false,
                            session_id: ping.session_id.clone(),
                        }),
                    );
                } else {
                    error!("User failed to respond, closing connection: {:?}", user_id2);
                    break;
                }
            }

            warn!("Sending ping to user: {:?}", user_id2);

            let response = SignalMessage::Ping(true, user_id2.clone(), None);
            let response = serde_json::to_string(&response).unwrap();
            if let Err(e) = tx2.send(Message::Text(response)) {
                error!("Websocket ping error: {}", e);
                break;
            }
        }
    });

    // Send messages to websocket from channel
    let mut send_task = tokio::spawn(async move {
        while let Some(message) = rx.next().await {
            if ws_send.send(message).await.is_err() {
                break;
            }
        }

        match ws_send
            .send(Message::Close(Some(CloseFrame {
                code: axum::extract::ws::close_code::NORMAL,
                reason: Cow::from("Goodbye"),
            })))
            .await
        {
            Ok(_) => info!("Sent close to {user_id}"),
            Err(e) => info!("Failed to close: {e}"),
        }
    });

    // Receive messages from websocket
    let connections2 = connections.clone();
    let sessions2 = sessions.clone();
    let pings2 = pings.clone();

    let mut recv_task = tokio::spawn(async move {
        while let Some(msg) = ws_recv.next().await {
            match msg {
                Ok(msg) => {
                    if let Err(err) = user_message(user_id, msg, &connections2, &sessions2, &pings2).await {
                        error!("error while handling user message: {}", err);
                    }
                }
                Err(e) => {
                    error!("Websocket error: {:?} {}", user_id, e);
                    break;
                }
            }
        }
    });

    connections.write().await.insert(user_id, tx);

    // Run all tasks and abort if any of them fails
    tokio::select! {
        t1 = (&mut send_task) => {
            match t1 {
                Ok(_) => info!("Sender task stopped"),
                Err(a) => info!("Error sending messages {a:?}")
            }
            recv_task.abort();
            ping_task.abort();
        },
        t2 = (&mut recv_task) => {
            match t2 {
                Ok(_) => info!("Receiver task stopped"),
                Err(b) => info!("Error receiving messages {b:?}")
            }
            send_task.abort();
            ping_task.abort();
        }
        t3 = (&mut ping_task) => {
            match t3 {
                Ok(_) => info!("Ping task stopped"),
                Err(c) => info!("Error pinging {c:?}")
            }
            send_task.abort();
            recv_task.abort();
        }
    }

    error!("User disconnected: {:?}", user_id);
    pings.lock().unwrap().remove(&user_id);
    user_disconnected(user_id, &connections, &sessions).await;
}

async fn user_message(sender_id: UserId, msg: Message, connections: &Connections, sessions: &Sessions, pings: &Pings) -> crate::Result<()> {
    if let Ok(msg) = msg.to_text() {
        if msg.is_empty() || msg == "ping" {
            // warn!("empty message from user {:?}", sender_id);
            return Ok(());
        }

        match serde_json::from_str::<SignalMessage>(msg) {
            Ok(request) => {
                info!("message received from user {:?}: {:?}", sender_id, request);
                match request {
                    SignalMessage::SessionJoin(session_id, is_host) => {
                        let mut sessions_writer = sessions.write().await;
                        let session = sessions_writer.entry(session_id.clone()).or_insert_with(Session::default);
                        let connections_reader = connections.read().await;

                        if is_host && session.host.is_none() {
                            session.host = Some(sender_id);
                            // start connections with all already present users
                            for client_id in &session.users {
                                {
                                    let host_tx = connections_reader.get(&sender_id).expect("host not in connections");
                                    let host_response = SignalMessage::SessionReady(session_id.clone(), *client_id);
                                    let host_response = serde_json::to_string(&host_response)?;
                                    host_tx.send(Message::Text(host_response)).expect("failed to send SessionReady message to host");
                                }
                            }
                        } else if is_host && session.host.is_some() {
                            warn!("connecting user wants to be a host, but host is already present, closing connection soon");

                            let connections2 = connections.clone();

                            tokio::task::spawn(async move {
                                let new_host_tx = {
                                    let connections_reader2 = connections2.read().await;
                                    connections_reader2.get(&sender_id).cloned()
                                };

                                tokio::time::sleep(Duration::from_secs(60)).await;
                                if let Some(new_host_tx) = new_host_tx {
                                    new_host_tx
                                        .send(Message::Close(Some(CloseFrame {
                                            code: 3001,
                                            reason: "Multiple hosts".into(),
                                        })))
                                        .expect("failed to send close message to host");
                                }
                            });
                        } else {
                            // connect new user with host
                            session.users.insert(sender_id);

                            if let Some(host_id) = session.host {
                                let host_tx = connections_reader.get(&host_id).expect("host not in connections");
                                let host_response = SignalMessage::SessionReady(session_id.clone(), sender_id);
                                let host_response = serde_json::to_string(&host_response)?;
                                host_tx.send(Message::Text(host_response)).expect("failed to send SessionReady message to host");
                            }
                        }
                    }
                    // pass offer to the other user in session without changing anything
                    SignalMessage::SdpOffer(session_id, recipient_id, offer) => {
                        let response = SignalMessage::SdpOffer(session_id, sender_id, offer);
                        let response = serde_json::to_string(&response)?;
                        let connections_reader = connections.read().await;
                        if let Some(recipient_tx) = connections_reader.get(&recipient_id) {
                            recipient_tx.send(Message::Text(response))?;
                        } else {
                            warn!("tried to send offer to non existing user");
                        }
                    }
                    // pass answer to the other user in session without changing anything
                    SignalMessage::SdpAnswer(session_id, recipient_id, answer) => {
                        let response = SignalMessage::SdpAnswer(session_id, sender_id, answer);
                        let response = serde_json::to_string(&response)?;
                        let connections_reader = connections.read().await;
                        if let Some(recipient_tx) = connections_reader.get(&recipient_id) {
                            recipient_tx.send(Message::Text(response))?;
                        } else {
                            warn!("tried to send offer to non existing user");
                        }
                    }
                    SignalMessage::IceCandidate(session_id, recipient_id, candidate) => {
                        let response = SignalMessage::IceCandidate(session_id, sender_id, candidate);
                        let response = serde_json::to_string(&response)?;
                        let connections_reader = connections.read().await;
                        let recipient_tx = connections_reader.get(&recipient_id).ok_or_else(|| anyhow!("no sender for given id"))?;

                        recipient_tx.send(Message::Text(response))?;
                    }
                    SignalMessage::Ping(is_host, recipient_id, session_id) => {
                        if is_host {
                            warn!("Received ping from user {:?}", recipient_id);
                            pings.lock().unwrap().insert(recipient_id.clone(), Arc::new(Ping { online: true, session_id }));
                        }
                    }
                    _ => {}
                }
            }
            Err(error) => {
                error!("An error occurred: {:?} {:?}", error, msg);
            }
        }
    }
    Ok(())
}

async fn user_disconnected(user_id: UserId, connections: &Connections, sessions: &Sessions) {
    connections.write().await.remove(&user_id);

    let mut session_to_delete = None;
    for (session_id, session) in sessions.write().await.iter_mut() {
        if session.host == Some(user_id) {
            session.host = None;
        } else if session.users.contains(&user_id) {
            session.users.remove(&user_id);
        }
        if session.host.is_none() && session.users.is_empty() {
            session_to_delete = Some(session_id.clone());
            break;
        }
    }
    // remove session if it's empty
    if let Some(session_id) = session_to_delete {
        sessions.write().await.remove(&session_id);
    }
}
