use std::fmt::Debug;

use crate::context::Context;
use crate::event::Event;
use crate::util::Id;

pub mod manager;
pub mod network;

pub trait Layer: Debug + Send + Sync {
    fn get_id(&self) -> Id;
    fn on_add(&mut self, context: &Context);
    fn on_remove(&mut self, context: &Context);
    fn filter_gather_events(&mut self, context: &Context, incoming_events: Vec<Box<dyn Event>>) -> Vec<Box<dyn Event>>;
}

pub trait AsLayer {
    fn as_layer(&self) -> &dyn Layer;
    fn as_boxed_layer(self: Box<Self>) -> Box<dyn Layer>;
}

#[typetag::serde(tag = "layer")]
pub trait NetworkLayer: AsLayer + Layer { }

impl<T: 'static + NetworkLayer> AsLayer for T {
    fn as_layer(&self) -> &dyn Layer {
        self
    }

    fn as_boxed_layer(self: Box<Self>) -> Box<dyn Layer> {
        self
    }
}

impl From<Box<dyn NetworkLayer>> for Box<dyn Layer> {
    fn from(network_layer: Box<dyn NetworkLayer>) -> Self {
        network_layer.as_boxed_layer()
    }
}