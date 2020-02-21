use crate::event::Event;
use crate::layer::Layer;
use crate::util::Id;

#[derive(Clone, Copy)]
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

pub struct RemoveLayerEvent {
    pub id: Id,
}

impl Event for RemoveLayerEvent { }