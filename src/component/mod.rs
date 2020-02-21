use crate::event::EventReceiver;
use crate::util::AsAny;

pub trait Component: AsAny + EventReceiver + Send {
    fn get_id(&self) -> String;
}