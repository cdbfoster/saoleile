use std::sync::{Arc, mpsc, Mutex, RwLock};

use crate::event::Event;
use crate::event::core::ShutdownEvent;
use crate::event::scene::SnapshotEvent;

use crate::scene::SceneStack;

pub fn scene_manager_event_thread(
    snapshot_queue: Arc<Mutex<Vec<SnapshotEvent>>>,
    scene_stack: Arc<RwLock<SceneStack>>,
    events_receiver: mpsc::Receiver<Box<dyn Event>>,
    outgoing_events_sender: mpsc::Sender<Box<dyn Event>>,
) {
    loop {
        let event = events_receiver.recv().unwrap();

        if event.as_any().is::<SnapshotEvent>() {
            //
        } else if event.as_any().is::<ShutdownEvent>() {
            break;
        }
    }
}