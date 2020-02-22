use serde::{Deserialize, Serialize};

use crate::component::Component;
use crate::event::NetworkEvent;
use crate::util::Id;

#[derive(Deserialize, Serialize)]
pub struct ComponentEvent {
    pub destination: Id,
    pub payload: Box<dyn NetworkEvent>,
}

#[typetag::serde]
impl NetworkEvent for ComponentEvent { }

#[derive(Deserialize, Serialize)]
pub struct AddComponentEvent {
    pub component: Box<dyn Component>,
}

#[typetag::serde]
impl NetworkEvent for AddComponentEvent { }

#[derive(Deserialize, Serialize)]
pub struct RemoveComponentEvent {
    pub id: Id,
}

#[typetag::serde]
impl NetworkEvent for RemoveComponentEvent { }