use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use crate::context::Context;
use crate::event::Event;
use crate::event::core::{ContextEvent, QuitEvent};
use crate::layer::Layer;

pub struct LayerManager {
    pub events: mpsc::Sender<Box<dyn Event>>,
    layers: Arc<RwLock<LayerStack>>,
    thread: Mutex<Option<thread::JoinHandle<()>>>,
}

impl LayerManager {
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::channel();
        let layers = Arc::new(RwLock::new(LayerStack::new()));

        let thread = Mutex::new(Some({
            let layers = layers.clone();
            thread::spawn(move || layer_manager_event_thread(receiver, layers))
        }));

        Self {
            events: sender,
            layers: layers,
            thread: thread,
        }
    }

    pub fn get_layer_by_name(&self, name: &str) -> Arc<RwLock<Box<dyn Layer>>> {
        self.layers.read().unwrap().get(name).expect("could not get layer")
    }
}

impl Drop for LayerManager {
    fn drop(&mut self) {
        self.events.send(Box::new(QuitEvent {})).ok();
        self.thread.lock().unwrap().take().unwrap().join().ok();
    }
}

fn layer_manager_event_thread(receiver: mpsc::Receiver<Box<dyn Event>>, layers: Arc<RwLock<LayerStack>>) {
    let mut layer_thread = None;
    let mut layer_thread_sender = None;

    loop {
        let event = receiver.recv().expect("LayerManager - event thread: can't receive event");

        if let Some(context_event) = event.as_any().downcast_ref::<ContextEvent>() {
            if layer_thread.is_none() {
                log!("LayerManager received a ContextEvent. Starting the layer thread.");
                let context = context_event.context.clone();

                let (sender, receiver) = mpsc::channel();
                layer_thread_sender = Some(sender);

                layer_thread = Some({
                    let layers = layers.clone();
                    thread::spawn(move || layer_manager_layer_thread(receiver, context, layers))
                });
            } else {
                log!(ERROR, "LayerManager received a ContextEvent, though the layer thread has already been started.");
            }
        } else if event.as_any().is::<QuitEvent>() {
            log!("LayerManager received a QuitEvent. Passing it to the layer thread.");
            if let Some(sender) = layer_thread_sender {
                sender.send(event).expect("LayerManager - event thread: can't send event");
            }
            break;
        }
    }

    if let Some(handle) = layer_thread {
        handle.join().unwrap();
    }
}

fn layer_manager_layer_thread(receiver: mpsc::Receiver<Box<dyn Event>>, context: Arc<Context>, layers: Arc<RwLock<LayerStack>>) {
    log!("LayerManager layer thread started");
    receiver.recv().expect("LayerManager - layer thread: can't receive event");
    log!("LayerManager layer thread ended");
}

struct LayerStack {
    layers: Vec<Arc<RwLock<Box<dyn Layer>>>>,
    name_map: HashMap<String, Arc<RwLock<Box<dyn Layer>>>>,
}

impl LayerStack {
    fn new() -> Self {
        Self {
            layers: Vec::new(),
            name_map: HashMap::new(),
        }
    }

    fn push(&mut self, layer: Box<dyn Layer>) -> Result<(), String> {
        let name = layer.get_name();

        if self.name_map.contains_key(&name) {
            return Err(format!("LayerStack::push: layer \"{}\" already exists", name));
        }

        let layer = Arc::new(RwLock::new(layer));

        self.layers.push(layer.clone());
        self.name_map.insert(name, layer);

        Ok(())
    }

    fn remove(&mut self, name: &str) -> Result<(), String> {
        let old_length = self.layers.len();

        self.layers.retain(|x| x.read().unwrap().get_name() != name);

        if self.layers.len() == old_length {
            Err(format!("LayerStack::remove: layer \"{}\" does not exist", name))
        } else {
            Ok(())
        }
    }

    fn get(&self, name: &str) -> Result<Arc<RwLock<Box<dyn Layer>>>, String> {
        if let Some(layer) = self.name_map.get(&String::from(name)) {
            Ok(layer.clone())
        } else {
            Err(format!("LayerStack::get: layer \"{}\" does not exist", name))
        }
    }
}