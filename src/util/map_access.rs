use crate::util::Id;

pub trait MapAccess<T> {
    fn get(&self, id: &Id) -> Result<&T, String>;
}