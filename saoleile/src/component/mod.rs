use std::fmt::Debug;

use crate::event::Event;
use crate::scene::{InterpolatedSceneStack, SceneStack};
use crate::util::{AsAny, Id};

#[typetag::serde(tag = "component")]
pub trait Component: AsAny + Debug + Send + Sync {
    fn get_id(&self) -> Id;
    fn interpolate_event(&self, scenes: &InterpolatedSceneStack, event: Box<dyn Event>, interpolation: f32) -> Box<dyn Component>;
    fn finalize_event(&mut self, scenes: &mut SceneStack, event: Box<dyn Event>);
    fn boxed_clone(&self) -> Box<dyn Component>;
}

impl Clone for Box<dyn Component> {
    fn clone(&self) -> Self {
        self.boxed_clone()
    }
}