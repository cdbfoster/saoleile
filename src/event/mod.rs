use crate::util::AsAny;

pub trait Event: AsAny + Send {
    fn is_network_event(&self) -> bool {
        false
    }

    fn as_boxed_network_event(self: Box<Self>) -> Option<Box<dyn NetworkEvent>> {
        None
    }
}

pub trait AsEvent {
    fn as_boxed_event(self: Box<Self>) -> Box<dyn Event>;
}

#[typetag::serde(tag = "event")]
pub trait NetworkEvent: AsEvent + Event { }

impl<T: 'static + NetworkEvent> AsEvent for T {
    fn as_boxed_event(self: Box<Self>) -> Box<dyn Event> {
        self
    }
}

impl<T: 'static + NetworkEvent> Event for T {
    fn is_network_event(&self) -> bool {
        true
    }

    fn as_boxed_network_event(self: Box<Self>) -> Option<Box<dyn NetworkEvent>> {
        Some(self)
    }
}

impl From<Box<dyn NetworkEvent>> for Box<dyn Event> {
    fn from(network_event: Box<dyn NetworkEvent>) -> Self {
        network_event.as_boxed_event()
    }
}

pub trait EventDispatcher {
    fn dispatch_event(&self, event: Box<dyn Event>);
}

pub trait EventReceiver {
    fn process_event(&mut self, event: Box<dyn Event>);
}

pub mod component;
pub mod core;
pub mod entity;
pub mod layer;
pub mod network;