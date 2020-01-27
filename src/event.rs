use crate::util::AsAny;

pub trait Event: AsAny + Send {

}

pub trait EventReceiver {
    fn process_event(&self, event: Box<dyn Event>);
}

pub mod core {
    use std::sync::Arc;

    use crate::context::Context;
    use crate::event::Event;

    pub struct ContextEvent {
        pub context: Arc<Context>,
    }

    impl Event for ContextEvent { }

    pub struct QuitEvent { }

    impl Event for QuitEvent { }
}