use crate::event::NetworkEvent;

pub trait CloneableEvent {
    fn boxed_clone(&self) -> Box<dyn NetworkEvent>;
}

impl<T: 'static + Clone + NetworkEvent> CloneableEvent for T {
    fn boxed_clone(&self) -> Box<dyn NetworkEvent> {
        Box::new(self.clone())
    }
}

impl Clone for Box<dyn NetworkEvent> {
    fn clone(&self) -> Self {
        self.boxed_clone()
    }
}