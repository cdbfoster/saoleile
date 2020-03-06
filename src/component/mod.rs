use std::fmt::Debug;

use crate::event::EventReceiver;
use crate::util::AsAny;

#[typetag::serde(tag = "component")]
pub trait Component: AsAny + Debug + EventReceiver + Send {
    fn get_id(&self) -> String;
}