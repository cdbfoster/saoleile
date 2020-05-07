use std::fmt::Debug;

use crate::util::{AsAny, Id};

#[typetag::serde(tag = "component")]
pub trait Component: AsAny + Debug + Send + Sync {
    fn get_id(&self) -> Id;
    fn boxed_clone(&self) -> Box<dyn Component>;
}

impl Clone for Box<dyn Component> {
    fn clone(&self) -> Self {
        self.boxed_clone()
    }
}