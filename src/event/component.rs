use serde::{Deserialize, Serialize};

use event_derive::NetworkEvent;

use crate::component::Component;
use crate::event::NetworkEvent as NetworkEventTrait;
use crate::util::Id;

#[derive(Clone, Debug, Deserialize, NetworkEvent, Serialize)]
pub struct ComponentEvent {
    pub scene_id: Id,
    pub entity_id: Id,
    pub component_id: Id,
    pub payload: Box<dyn NetworkEventTrait>,
}

#[derive(Clone, Debug, Deserialize, NetworkEvent, Serialize)]
pub struct AddComponentEvent {
    pub scene_id: Id,
    pub entity_id: Id,
    pub component: Box<dyn Component>,
}

#[derive(Clone, Debug, Deserialize, NetworkEvent, Serialize)]
pub struct RemoveComponentEvent {
    pub scene_id: Id,
    pub entity_id: Id,
    pub component_id: Id,
}