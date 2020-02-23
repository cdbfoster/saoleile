use serde::{Deserialize, Serialize};

use crate::event::{Event, NetworkEvent};
use crate::layer::{Layer, NetworkLayer};
use crate::util::Id;

#[derive(Clone, Copy, Deserialize, Serialize)]
pub enum LayerPosition {
    Top,
    Bottom,
}

pub struct AddLayerEvent {
    pub push: LayerPosition,
    pub pin: Option<LayerPosition>,
    pub layer: Box<dyn Layer>,
}

impl Event for AddLayerEvent { }

#[derive(Deserialize, Serialize)]
pub struct AddNetworkLayerEvent {
    pub push: LayerPosition,
    pub pin: Option<LayerPosition>,
    pub layer: Box<dyn NetworkLayer>,
}

#[typetag::serde]
impl NetworkEvent for AddNetworkLayerEvent { }

#[derive(Deserialize, Serialize)]
pub struct RemoveLayerEvent {
    pub id: Id,
}

#[typetag::serde]
impl NetworkEvent for RemoveLayerEvent { }