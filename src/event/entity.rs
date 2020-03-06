use serde::{Deserialize, Serialize};

use crate::entity::Entity;
use crate::event::NetworkEvent;
use crate::util::Id;

#[derive(Debug, Deserialize, Serialize)]
pub struct EntityEvent {
    pub destination: Id,
    pub payload: Box<dyn NetworkEvent>,
}

#[typetag::serde]
impl NetworkEvent for EntityEvent { }

#[derive(Debug, Deserialize, Serialize)]
pub struct AddEntityEvent {
    pub entity: Entity,
}

#[typetag::serde]
impl NetworkEvent for AddEntityEvent { }

#[derive(Debug, Deserialize, Serialize)]
pub struct RemoveEntityEvent {
    pub id: Id,
}

#[typetag::serde]
impl NetworkEvent for RemoveEntityEvent { }