use serde::{Deserialize, Serialize};

use crate::event::NetworkEvent;
use crate::layer::Layer;
use crate::util::Id;

#[derive(Clone, Copy, Deserialize, Serialize)]
pub enum LayerPosition {
    Top,
    Bottom,
}

#[derive(Deserialize, Serialize)]
pub struct AddLayerEvent {
    pub push: LayerPosition,
    pub pin: Option<LayerPosition>,
    pub layer: Box<dyn Layer>,
}

#[typetag::serde]
impl NetworkEvent for AddLayerEvent { }

#[derive(Deserialize, Serialize)]
pub struct RemoveLayerEvent {
    pub id: Id,
}

#[typetag::serde]
impl NetworkEvent for RemoveLayerEvent { }