use std::fmt::Debug;

use crate::util::AsAny;

#[typetag::serde(tag = "component")]
pub trait Component: AsAny + Debug + Send + Sync {
    fn get_id(&self) -> String;
}