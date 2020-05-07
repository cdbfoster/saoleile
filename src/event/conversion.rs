use crate::event::{Event, NetworkEvent};

pub trait AsEvent {
    fn as_event(self: Box<Self>) -> Box<dyn Event>;
}

impl<T: 'static + Event> AsEvent for T {
    fn as_event(self: Box<Self>) -> Box<dyn Event> {
        self
    }
}

pub trait AsNetworkEvent {
    fn is_network_event(&self) -> bool;
    fn as_network_event(self: Box<Self>) -> Option<Box<dyn NetworkEvent>>;
}

impl From<Box<dyn NetworkEvent>> for Box<dyn Event> {
    fn from(network_event: Box<dyn NetworkEvent>) -> Self {
        network_event.as_event()
    }
}