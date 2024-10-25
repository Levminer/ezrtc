use std::fmt::{Display, Formatter};
use std::str::FromStr;

use serde::{Deserialize, Serialize};

/// Unique identifier of signaling session that each user provides
/// when communicating with the signaling server.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, Hash)]
pub struct SessionId(String);

impl SessionId {
    /// Wrap String into a `SessionId` `struct`
    pub fn new(inner: String) -> Self {
        SessionId(inner)
    }

    /// Return reference to the underling string
    pub fn as_str(&self) -> &str {
        &self.0
    }
    /// Acquire the underlying type
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl FromStr for SessionId {
    type Err = Box<dyn std::error::Error>;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(SessionId(s.to_string()))
    }
}

impl Display for SessionId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Unique identifier of each peer connected to signaling server
/// useful when communicating in one-to-many and many-to-many .
#[derive(Debug, Copy, Clone, Eq, PartialEq, Serialize, Deserialize, Hash)]
pub struct UserId(usize);

impl UserId {
    /// Wrap `usize` into a `UserId` `struct`
    pub fn new(inner: usize) -> Self {
        UserId(inner)
    }

    /// Acquire the underlying type
    pub fn into_inner(self) -> usize {
        self.0
    }
}

impl From<usize> for UserId {
    fn from(val: usize) -> Self {
        UserId(val)
    }
}

impl Display for UserId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Unique identifier specifying which peer is host and will be creating an offer,
/// and which will await it.
pub type IsHost = bool;

/// The ice candidate sent from the user to the host.
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct IceCandidateJSON {
    pub candidate: String,
    #[serde(rename = "sdpMid")]
    pub sdp_mid: Option<String>,
    #[serde(rename = "sdpMLineIndex")]
    pub sdp_mline_index: Option<u16>,
    #[serde(rename = "usernameFragment")]
    pub username_fragment: Option<String>,
}

/// `Enum` consisting of two main categories are messages used to setup signaling session
/// and messages used to setup `WebRTC` connection afterwards.
/// Most of the include [`SessionId`] and [`UserId`] to uniquely identify each peer.
#[derive(Debug, Serialize, Deserialize)]
pub enum SignalMessage {
    /// Either client or server connecting to signaling session
    SessionJoin(SessionId, IsHost),

    /// Report back to the users that both of them are in session
    SessionReady(SessionId, UserId),

    /// `SDP` Offer that gets passed to the other user without modifications
    SdpOffer(SessionId, UserId, String),

    /// `SDP` Answer that gets passed to the other user without modifications
    SdpAnswer(SessionId, UserId, String),

    /// Proposed ICE Candidate of one user passed to the other user without modifications
    IceCandidate(SessionId, UserId, String),

    /// Generic error containing detailed information about the cause
    Error(SessionId, UserId, String),

    /// Ping message
    Ping(bool, UserId, Option<SessionId>),
}
