use crate::context::Context;
use crate::event::Event;

pub mod manager;

pub trait Layer: Send + Sync {
    fn get_name(&self) -> String;
    fn on_add(&mut self, context: &Context);
    fn on_remove(&mut self, context: &Context);
    fn filter_gather_events(&mut self, context: &Context, incoming_events: Vec<Box<dyn Event>>) -> Vec<Box<dyn Event>>;
}