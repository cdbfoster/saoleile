use crate::event::EventReceiver;
use crate::util::AsAny;

#[typetag::serde(tag = "component")]
pub trait Component: AsAny + EventReceiver + Send {
    fn get_id(&self) -> String;
}