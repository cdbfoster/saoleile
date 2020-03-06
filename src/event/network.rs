use std::net::SocketAddr;

use serde::{Deserialize, Serialize};

use crate::event::NetworkEvent;

#[derive(Debug, Deserialize, Serialize)]
pub struct DisconnectEvent { }

#[typetag::serde]
impl NetworkEvent for DisconnectEvent { }

#[derive(Debug, Deserialize, Serialize)]
pub struct DroppedNetworkEvent {
    pub recipient: SocketAddr,
    pub event: Box<dyn NetworkEvent>,
}

#[typetag::serde]
impl NetworkEvent for DroppedNetworkEvent { }