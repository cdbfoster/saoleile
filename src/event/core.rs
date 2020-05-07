use std::sync::Arc;

use serde::{Deserialize, Serialize};

use event_derive::{Event, NetworkEvent};

use crate::context::Context;

#[derive(Debug, Event)]
pub struct ContextEvent {
    pub context: Arc<Context>,
}

#[derive(Debug, Event)]
pub struct QuitEvent { }

#[derive(Clone, Debug, Deserialize, NetworkEvent, Serialize)]
pub struct ShutdownEvent { }