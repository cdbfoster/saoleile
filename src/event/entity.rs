use crate::entity::Entity;
use crate::event::Event;
use crate::util::Id;

pub struct EntityEvent {
    pub destination: Id,
    pub payload: Box<dyn Event>,
}

impl Event for EntityEvent { }

pub struct AddEntityEvent {
    pub entity: Entity,
}

impl Event for AddEntityEvent { }

pub struct RemoveEntityEvent {
    pub id: Id,
}

impl Event for RemoveEntityEvent { }