use std::fmt::Debug;

use crate::util::AsAny;
use self::conversion::{AsEvent, AsNetworkEvent};
use self::util::CloneableEvent;

pub trait Event: AsAny + AsEvent + AsNetworkEvent + Debug + Send { }

#[typetag::serde(tag = "event")]
pub trait NetworkEvent: CloneableEvent + Event { }

impl<T: 'static + NetworkEvent> Event for T { }

pub mod component;
pub mod core;
pub mod layer;
pub mod network;

pub mod conversion;
mod util;