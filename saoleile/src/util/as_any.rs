use std::any::Any;

pub trait AsAny {
    fn as_any(&self) -> &dyn Any;

    fn as_boxed_any(self: Box<Self>) -> Box<dyn Any>;
}

impl<T: Any> AsAny for T {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_boxed_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }
}