use std::sync::Arc;

use crate::context::Context;
use crate::event::Event;

#[derive(Debug)]
pub struct ContextEvent {
    pub context: Arc<Context>,
}

impl Event for ContextEvent { }

#[derive(Debug)]
pub struct QuitEvent { }

impl Event for QuitEvent { }