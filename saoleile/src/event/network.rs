use std::net::SocketAddr;

use serde::{Deserialize, Serialize};

use saoleile_derive::NetworkEvent;

use crate::event::NetworkEvent;

#[derive(Clone, Debug, Deserialize, NetworkEvent, Serialize)]
pub struct DisconnectEvent { }

#[derive(Clone, Debug, Deserialize, NetworkEvent, Serialize)]
pub struct DroppedNetworkEvent {
    pub recipient: SocketAddr,
    pub event: Box<dyn NetworkEvent>,
}