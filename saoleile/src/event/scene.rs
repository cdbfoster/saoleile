use serde::{Deserialize, Serialize};

use saoleile_derive::NetworkEvent;

use crate::event::NetworkEvent;
use crate::scene::{Entity, Scene};
use crate::util::{Tick, Id};

#[derive(Clone, Debug, Deserialize, NetworkEvent, Serialize)]
pub struct SnapshotEvent {
    pub authority: Id,
    pub is_correction: bool,
    pub tick: Tick,
    pub events: Vec<Box<dyn NetworkEvent>>,
}

#[derive(Clone, Debug, Deserialize, NetworkEvent, Serialize)]
pub struct AddSceneEvent {
    pub scene_id: Id,
    pub scene: Scene,
}

#[derive(Clone, Debug, Deserialize, NetworkEvent, Serialize)]
pub struct RemoveSceneEvent {
    pub scene_id: Id,
}

#[derive(Clone, Debug, Deserialize, NetworkEvent, Serialize)]
pub struct AddEntityEvent {
    pub scene_id: Id,
    pub entity_id: Id,
    pub entity: Entity,
}

#[derive(Clone, Debug, Deserialize, NetworkEvent, Serialize)]
pub struct RemoveEntityEvent {
    pub scene_id: Id,
    pub entity_id: Id,
}