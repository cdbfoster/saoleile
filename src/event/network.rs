use std::net::SocketAddr;

use serde::{Deserialize, Serialize};

use crate::event::NetworkEvent;

#[derive(Deserialize, Serialize)]
pub struct DroppedNetworkEvent {
    pub recipient: SocketAddr,
    pub event: Box<dyn NetworkEvent>,
}

#[typetag::serde]
impl NetworkEvent for DroppedNetworkEvent { }