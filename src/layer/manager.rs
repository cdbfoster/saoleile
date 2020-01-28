use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use crate::context::Context;
use crate::event::{Event, EventDispatcher};
use crate::event::core::{ContextEvent, QuitEvent};
use crate::layer::{AddLayerEvent, Layer, LayerPosition, RemoveLayerEvent};
use crate::util::{DynIter, Id, MapAccess};
use crate::util::view_lock::{LockedView, LockedViewMut, ViewLock};

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

    pub fn shutdown(&self) {
        let mut running = self.running.lock().unwrap();

        if *running {
            self.dispatch_event(Box::new(QuitEvent {}));
            self.thread.lock().unwrap().take().unwrap().join().ok();
            *running = false;
        }
    }
}

impl EventDispatcher for LayerManager {
    fn dispatch_event(&self, event: Box<dyn Event>) {
        self.events.lock().unwrap().send(event).ok();
    }
}

impl<'a> ViewLock<'a, Box<dyn Layer>, LayerStack> for LayerManager {
    fn lock_view(&'a self) -> LockedView<'a, Box<dyn Layer>, LayerStack> {
        LockedView::new(self.layers.read().unwrap())
    }

    fn lock_view_mut(&'a self) -> LockedViewMut<'a, Box<dyn Layer>, LayerStack> {
        LockedViewMut::new(self.layers.write().unwrap())
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

    let mut context: Option<Arc<Context>> = None;

    loop {
        let event = receiver.recv().expect("LayerManager event thread: can't receive event");

        if event.as_any().is::<AddLayerEvent>() {
            let add_layer_event = *event.as_boxed_any().downcast::<AddLayerEvent>().unwrap();

            let mut layers = layers.write().unwrap();
            match layers.push(add_layer_event.push, add_layer_event.pin, add_layer_event.layer) {
                Ok(layer) => layer.write().unwrap().on_add(context.as_ref().unwrap()),
                Err(message) => log!(ERROR, "LayerManager event thread error: {}", message),
            }
        } else if event.as_any().is::<RemoveLayerEvent>() {
            let remove_layer_event = *event.as_boxed_any().downcast::<RemoveLayerEvent>().unwrap();

            let mut layers = layers.write().unwrap();
            match layers.remove(&remove_layer_event.id) {
                Ok(mut layer) => layer.on_remove(context.as_ref().unwrap()),
                Err(message) => log!(ERROR, "LayerManager event thread error: {}", message),
            }
        } else if event.as_any().is::<ContextEvent>() {
            let context_event = *event.as_boxed_any().downcast::<ContextEvent>().unwrap();

            if layer_thread.is_none() {
                log!("LayerManager event thread received a ContextEvent. Starting the layer thread.");

                context = Some(context_event.context.clone());

                let (sender, receiver) = mpsc::channel();
                layer_thread_sender = Some(sender);

                layer_thread = Some(thread::spawn(move || layer_manager_layer_thread(receiver, context_event.context)));
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
            let mut passed_events = Vec::new();
            for mut layer in context.layer_manager.lock_view_mut().iter_mut() {
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

struct LayerStack {
    layers: Vec<Arc<RwLock<Box<dyn Layer>>>>,
    last_top: Option<usize>,
    first_bottom: Option<usize>,
    id_map: HashMap<Id, Arc<RwLock<Box<dyn Layer>>>>,
}

impl LayerStack {
    fn new() -> Self {
        Self {
            layers: Vec::new(),
            last_top: None,
            first_bottom: None,
            id_map: HashMap::new(),
        }
    }

    fn push(&mut self, position: LayerPosition, pin: Option<LayerPosition>, layer: Box<dyn Layer>) -> Result<&Arc<RwLock<Box<dyn Layer>>>, String> {
        let id = layer.get_id();

        if self.id_map.contains_key(&id) {
            return Err(format!("LayerStack::push: layer \"{}\" already exists", id));
        }

        let layer = Arc::new(RwLock::new(layer));
        self.id_map.insert(id.clone(), layer.clone());

        if let Some(pin) = pin {
            match pin {
                LayerPosition::Top => if let Some(last_top) = self.last_top {
                    let i = match position {
                        LayerPosition::Top => 0,
                        LayerPosition::Bottom => last_top + 1,
                    };
                    self.layers.insert(i, layer);
                    self.last_top = Some(last_top + 1);
                } else {
                    self.layers.insert(0, layer);
                    self.last_top = Some(0);
                },
                LayerPosition::Bottom => if let Some(first_bottom) = self.first_bottom {
                    let i = match position {
                        LayerPosition::Top => first_bottom,
                        LayerPosition::Bottom => self.layers.len(),
                    };
                    self.layers.insert(i, layer);
                } else {
                    self.first_bottom = Some(self.layers.len());
                    self.layers.push(layer);
                },
            }
        } else {
            let i = match position {
                LayerPosition::Top => self.last_top.map_or(0, |i| i + 1),
                LayerPosition::Bottom => self.first_bottom.unwrap_or(self.layers.len()),
            };
            self.layers.insert(i, layer);
            self.first_bottom = self.first_bottom.map(|i| i + 1);
        }

        self.get(&id)
    }

    fn remove(&mut self, id: &Id) -> Result<Box<dyn Layer>, String> {
        if !self.id_map.contains_key(id) {
            return Err(format!("LayerStack::remove: layer \"{}\" does not exist", id))
        }

        let index = self.layers.iter().enumerate().find(|&(_, x)| x.read().unwrap().get_id() == *id).map(|(i, _)| i);
        if index.is_none() {
            panic!("LayerStack is in an inconsistent state! Layer \"{}\" exists in id_map but not layers", id);
        }

        self.id_map.remove(id);
        Ok(Arc::try_unwrap(self.layers.remove(index.unwrap())).unwrap().into_inner().unwrap())
    }
}

impl MapAccess<Arc<RwLock<Box<dyn Layer>>>> for LayerStack {
    fn get(&self, id: &Id) -> Result<&Arc<RwLock<Box<dyn Layer>>>, String> {
        if let Some(layer) = self.id_map.get(id) {
            Ok(layer)
        } else {
            Err(format!("LayerStack::get: layer \"{}\" does not exist", id))
        }
    }
}

impl<'a> DynIter<'a> for LayerStack {
    type Item = Arc<RwLock<Box<dyn Layer>>>;

    fn dyn_iter(&'a self) -> Box<dyn Iterator<Item = &'a Self::Item> + 'a> {
        Box::new(self.layers.iter())
    }
}