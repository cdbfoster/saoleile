use std::fmt::Debug;

use crate::context::Context;
use crate::event::Event;
use crate::util::Id;

pub mod manager;

pub trait Layer: Debug + Send + Sync {
    fn get_id(&self) -> Id;
    fn on_add(&mut self, context: &Context);
    fn on_remove(&mut self, context: &Context);
    fn filter_gather_events(&mut self, context: &Context, incoming_events: Vec<Box<dyn Event>>) -> Vec<Box<dyn Event>>;
}

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