use crate::event::{Event, EventReceiver};
use crate::util::{AsAny, Id};

pub trait Component: AsAny + EventReceiver + Send {
    fn get_id(&self) -> String;
}

pub struct ComponentEvent {
    pub destination: Id,
    pub payload: Box<dyn Event>,
}

impl Event for ComponentEvent { }

pub struct AddComponentEvent {
    pub component: Box<dyn Component>,
}

impl Event for AddComponentEvent { }

pub struct RemoveComponentEvent {
    pub id: Id,
}

impl Event for RemoveComponentEvent { }