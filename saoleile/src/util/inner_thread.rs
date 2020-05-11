use std::sync::Mutex;
use std::thread;

#[derive(Debug)]
pub struct InnerThread(Mutex<Option<thread::JoinHandle<()>>>);

impl InnerThread {
    pub fn new(join_handle: thread::JoinHandle<()>) -> Self {
        Self(Mutex::new(Some(join_handle)))
    }

    pub fn join(&self) {
        self.0.lock().unwrap().take().unwrap().join().ok();
    }
}
