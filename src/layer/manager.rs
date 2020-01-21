use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock, RwLockReadGuard};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use crate::context::Context;
use crate::event::Event;
use crate::event::core::{ContextEvent, QuitEvent};
use crate::layer::Layer;

pub struct LayerManager {
    events: Mutex<mpsc::Sender<Box<dyn Event>>>,
    layers: Arc<RwLock<LayerStack>>,
    thread: Mutex<Option<thread::JoinHandle<()>>>,
    running: Mutex<bool>,
}

impl LayerManager {
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::channel();
        let layers = Arc::new(RwLock::new(LayerStack::new()));

        let thread = {
            let layers = layers.clone();
            thread::spawn(move || layer_manager_event_thread(receiver, layers))
        };

        Self {
            events: Mutex::new(sender),
            layers: layers,
            thread: Mutex::new(Some(thread)),
            running: Mutex::new(true),
        }
    }

    pub fn send_event(&self, event: Box<dyn Event>) {
        self.events.lock().unwrap().send(event).ok();
    }

    pub fn get_layers(&self) -> RwLockReadGuard<LayerStack> {
        self.layers.read().unwrap()
    }

    pub fn shutdown(&self) {
        let mut running = self.running.lock().unwrap();

        if *running {
            self.send_event(Box::new(QuitEvent {}));
            self.thread.lock().unwrap().take().unwrap().join().ok();
            *running = false;
        }
    }
}

impl Drop for LayerManager {
    fn drop(&mut self) {
        self.shutdown();
    }
}

fn layer_manager_event_thread(receiver: mpsc::Receiver<Box<dyn Event>>, layers: Arc<RwLock<LayerStack>>) {
    let mut layer_thread = None;
    let mut layer_thread_sender = None;

    loop {
        let event = receiver.recv().expect("LayerManager event thread: can't receive event");

        if let Some(context_event) = event.as_any().downcast_ref::<ContextEvent>() {
            if layer_thread.is_none() {
                log!("LayerManager event thread received a ContextEvent. Starting the layer thread.");
                let context = context_event.context.clone();

                let (sender, receiver) = mpsc::channel();
                layer_thread_sender = Some(sender);

                layer_thread = Some(thread::spawn(move || layer_manager_layer_thread(receiver, context)));
            } else {
                log!(ERROR, "LayerManager event thread received a ContextEvent, though the layer thread has already been started.");
            }
        } else if event.as_any().is::<QuitEvent>() {
            log!("LayerManager event thread received a QuitEvent. Passing it to the layer thread.");
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

fn layer_manager_layer_thread(receiver: mpsc::Receiver<Box<dyn Event>>, context: Arc<Context>) {
    const ITERATIONS_PER_SECOND: u32 = 20;
    const ITERATION_NS: u32 = 1_000_000_000 / ITERATIONS_PER_SECOND;

    let mut previous_time = Instant::now();
    let mut accumulated_time = 0;

    loop {
        let current_time = Instant::now();
        let delta_time = current_time.duration_since(previous_time);
        previous_time = current_time;
        accumulated_time += delta_time.as_nanos();

        if let Ok(event) = receiver.try_recv() {
            if event.as_any().is::<QuitEvent>() {
                log!("LayerManager layer thread received a QuitEvent.");
                break;
            }
        }

        if accumulated_time > ITERATION_NS as u128 {
            let layer_stack_guard = context.layer_manager.get_layers();
            let layer_stack = &*layer_stack_guard;

            let mut passed_events = Vec::new();

            for layer in layer_stack.layers.iter() {
                let mut layer_guard = layer.write().unwrap();
                let layer = &mut *layer_guard;

                passed_events = layer.filter_gather_events(&context, passed_events);
            }

            // The exact number of iterations we run isn't important, so just clear the counter.
            accumulated_time = 0;
        }

        let current_time = Instant::now();
        let delta_time = current_time.duration_since(previous_time);
        previous_time = current_time;
        accumulated_time += delta_time.as_nanos();

        // Sleep for half the remaining time.
        // Sleeping is never exact, so we don't want to overshoot.
        if accumulated_time < ITERATION_NS as u128 {
            thread::sleep(Duration::from_nanos((ITERATION_NS as u64 - accumulated_time as u64) / 2));
        }
    }
}

pub struct LayerStack {
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

    pub fn get(&self, name: &str) -> Result<Arc<RwLock<Box<dyn Layer>>>, String> {
        if let Some(layer) = self.name_map.get(&String::from(name)) {
            Ok(layer.clone())
        } else {
            Err(format!("LayerStack::get: layer \"{}\" does not exist", name))
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = &Arc<RwLock<Box<dyn Layer>>>> {
        self.layers.iter()
    }
}