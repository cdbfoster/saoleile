use crate::component::Component;
use crate::event::Event;
use crate::util::Id;

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