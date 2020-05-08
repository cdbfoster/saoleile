use serde::{Deserialize, Serialize};

use saoleile_derive::{Event, NetworkEvent};

use crate::layer::{Layer, NetworkLayer};
use crate::util::Id;

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub enum LayerPosition {
    Top,
    Bottom,
}

#[derive(Debug, Event)]
pub struct AddLayerEvent {
    pub push: LayerPosition,
    pub pin: Option<LayerPosition>,
    pub layer: Box<dyn Layer>,
}

#[derive(Clone, Debug, Deserialize, NetworkEvent, Serialize)]
pub struct AddNetworkLayerEvent {
    pub push: LayerPosition,
    pub pin: Option<LayerPosition>,
    pub layer: Box<dyn NetworkLayer>,
}

#[derive(Clone, Debug, Deserialize, NetworkEvent, Serialize)]
pub struct RemoveLayerEvent {
    pub id: Id,
}