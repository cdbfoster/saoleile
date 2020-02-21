use crate::util::AsAny;

pub trait Event: AsAny + Send { }

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