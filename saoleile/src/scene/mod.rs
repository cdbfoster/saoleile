use std::collections::{HashMap, HashSet};
use std::sync::{Arc, mpsc, Mutex, RwLock, RwLockReadGuard};
use std::thread;

use crate::component::Component;
use crate::event::Event;
use crate::event::component::{AddComponentEvent, ComponentEvent, RemoveComponentEvent};
use crate::event::core::ShutdownEvent;
use crate::event::scene::{AddEntityEvent, AddSceneEvent, RemoveEntityEvent, RemoveSceneEvent, SnapshotEvent};
use crate::util::{Id, InnerThread, Time};

use self::event_thread::scene_manager_event_thread;

#[derive(Debug)]
pub struct SceneManager {
    snapshot_queue: Arc<Mutex<Vec<SnapshotEvent>>>,
    scene_stack: Arc<RwLock<SceneStack>>,

    events: Mutex<mpsc::Sender<Box<dyn Event>>>,
    receiver: Mutex<mpsc::Receiver<Box<dyn Event>>>, // For notifying about things like client input mispredictions
    event_thread: InnerThread,
    running: Mutex<bool>,
}

impl SceneManager {
    pub fn new() -> Self {
        let snapshot_queue = Arc::new(Mutex::new(Vec::new()));
        let scene_stack = Arc::new(RwLock::new(SceneStack::new()));

        let (events, events_receiver) = mpsc::channel();
        let (outgoing_events_sender, receiver) = mpsc::channel();

        let event_thread = InnerThread::new({
            let snapshot_queue = snapshot_queue.clone();
            let scene_stack = scene_stack.clone();

            thread::Builder::new()
                .name("SceneManager event thread".to_string())
                .spawn(move || scene_manager_event_thread(
                    snapshot_queue,
                    scene_stack,
                    events_receiver,
                    outgoing_events_sender,
                )).unwrap()
        });

        Self {
            snapshot_queue,
            scene_stack,
            events: Mutex::new(events),
            receiver: Mutex::new(receiver),
            event_thread,
            running: Mutex::new(true),
        }
    }

    pub fn process_event(&self, event: Box<dyn Event>) {
        let guard = self.events.lock().unwrap();
        guard.send(event).unwrap();
    }

    pub fn interpolate(&self, interpolation_time: Time) -> InterpolatedSceneStack {
        let scene_stack_guard = self.scene_stack.read().unwrap();
        let snapshot_queue_guard = self.snapshot_queue.lock().unwrap();

        let mut interpolation = InterpolatedSceneStack {
            time: Time::new(0, 0.0),
            base: scene_stack_guard,
            interpolated: HashMap::new(),
            removed_scenes: HashSet::new(),
            removed_entities: HashMap::new(),
            removed_components: HashMap::new(),
        };

        'iter_snapshot: for (i, snapshot) in snapshot_queue_guard.iter().enumerate() {
            if snapshot.tick <= interpolation_time {
                interpolation.time = snapshot.tick.into();

                for event in snapshot.events.iter() {
                    if let Some(add_scene_event) = event.as_any().downcast_ref::<AddSceneEvent>() {
                        interpolation.add_scene(
                            &add_scene_event.scene_id,
                            add_scene_event.scene.clone(),
                        );
                    } else if let Some(remove_scene_event) = event.as_any().downcast_ref::<RemoveSceneEvent>() {
                        interpolation.remove_scene(
                            &remove_scene_event.scene_id,
                        );
                    } else if let Some(add_entity_event) = event.as_any().downcast_ref::<AddEntityEvent>() {
                        interpolation.add_entity(
                            &add_entity_event.scene_id,
                            &add_entity_event.entity_id,
                            add_entity_event.entity.clone(),
                        );
                    } else if let Some(remove_entity_event) = event.as_any().downcast_ref::<RemoveEntityEvent>() {
                        interpolation.remove_entity(
                            &remove_entity_event.scene_id,
                            &remove_entity_event.entity_id,
                        );
                    } else if let Some(add_component_event) = event.as_any().downcast_ref::<AddComponentEvent>() {
                        interpolation.add_component(
                            &add_component_event.scene_id,
                            &add_component_event.entity_id,
                            add_component_event.component.clone(),
                        );
                    } else if let Some(remove_component_event) = event.as_any().downcast_ref::<RemoveComponentEvent>() {
                        interpolation.remove_component(
                            &remove_component_event.scene_id,
                            &remove_component_event.entity_id,
                            &remove_component_event.component_id,
                        );
                    } else if let Some(component_event) = event.as_any().downcast_ref::<ComponentEvent>() {
                        interpolation.interpolate_component_event(
                            &component_event.scene_id,
                            &component_event.entity_id,
                            &component_event.component_id,
                            component_event.payload.clone().into(),
                            1.0,
                        );
                    }
                }
            } else if i > 0 {
                let previous_snapshot = &snapshot_queue_guard[i - 1];

                if previous_snapshot.tick > interpolation_time {
                    continue 'iter_snapshot;
                }

                interpolation.time = interpolation_time;
                let interpolation_amount = (interpolation_time - previous_snapshot.tick).as_f32() / (snapshot.tick - previous_snapshot.tick) as f32;

                for event in snapshot.events.iter() {
                    if let Some(component_event) = event.as_any().downcast_ref::<ComponentEvent>() {
                        interpolation.interpolate_component_event(
                            &component_event.scene_id,
                            &component_event.entity_id,
                            &component_event.component_id,
                            component_event.payload.clone().into(),
                            interpolation_amount,
                        );
                    }
                }

                break 'iter_snapshot;
            }
        }

        interpolation
    }

    pub fn shutdown(&self) {
        let mut running = self.running.lock().unwrap();

        if *running {
            // Send a shutdown event to the event thread
            self.process_event(Box::new(ShutdownEvent { }));
            self.event_thread.join();
            *running = false;
        }
    }
}

impl Drop for SceneManager {
    fn drop(&mut self) {
        self.shutdown();
    }
}

pub struct InterpolatedSceneStack<'a> {
    time: Time,
    base: RwLockReadGuard<'a, SceneStack>,
    interpolated: HashMap<Id, Scene>,
    removed_scenes: HashSet<Id>,
    removed_entities: HashMap<Id, HashSet<Id>>,
    removed_components: HashMap<Id, HashMap<Id, HashSet<Id>>>,
}

impl<'a> InterpolatedSceneStack<'a> {
    fn add_scene(&mut self, scene_id: &Id, scene: Scene) {
        self.removed_scenes.remove(scene_id);

        for (entity_id, entity) in scene.iter() {
            self.removed_entities
                .get_mut(scene_id)
                .map(|s| s.remove(entity_id));

            for component_id in entity.keys() {
                self.removed_components
                    .get_mut(scene_id)
                    .and_then(|s| s.get_mut(entity_id))
                    .map(|e| e.remove(component_id));
            }
        }

        self.interpolated.insert(
            scene_id.clone(),
            scene,
        );
    }

    fn remove_scene(&mut self, scene_id: &Id) {
        let removed_scene = self.interpolated.remove(scene_id);
        let scene = removed_scene.as_ref().or(self.base.scenes.get(scene_id));

        if let Some(scene) = scene {
            self.removed_scenes.insert(scene_id.clone());

            for (entity_id, entity) in scene.iter() {
                let scene_entry = self.removed_entities.entry(scene_id.clone()).or_insert(HashSet::new());
                scene_entry.insert(entity_id.clone());

                for component_id in entity.keys() {
                    let scene_entry = self.removed_components.entry(scene_id.clone()).or_insert(HashMap::new());
                    let entity_entry = scene_entry.entry(entity_id.clone()).or_insert(HashSet::new());
                    entity_entry.insert(component_id.clone());
                }
            }
        }
    }

    fn add_entity(&mut self, scene_id: &Id, entity_id: &Id, entity: Entity) {
        self.removed_entities
            .get_mut(scene_id)
            .map(|s| s.remove(entity_id));

        for component_id in entity.keys() {
            self.removed_components
                .get_mut(scene_id)
                .and_then(|s| s.get_mut(entity_id))
                .map(|e| e.remove(component_id));
        }

        let scene_entry = self.interpolated.entry(scene_id.clone()).or_insert(Scene::new());
        scene_entry.insert(entity_id.clone(), entity);
    }

    fn remove_entity(&mut self, scene_id: &Id, entity_id: &Id) {
        let removed_entity = self.interpolated
            .get_mut(scene_id)
            .and_then(|s| s.remove(entity_id));
        let entity = removed_entity.as_ref().or(self.base.scenes
            .get(scene_id)
            .and_then(|s| s.get(entity_id))
        );

        let scene_entry = self.removed_entities.entry(scene_id.clone()).or_insert(HashSet::new());
        scene_entry.insert(entity_id.clone());

        if let Some(entity) = entity {
            for component_id in entity.keys() {
                let scene_entry = self.removed_components.entry(scene_id.clone()).or_insert(HashMap::new());
                let entity_entry = scene_entry.entry(entity_id.clone()).or_insert(HashSet::new());
                entity_entry.insert(component_id.clone());
            }
        }
    }

    fn add_component(&mut self, scene_id: &Id, entity_id: &Id, component: Box<dyn Component>) {
        self.removed_components
            .get_mut(scene_id)
            .and_then(|s| s.get_mut(entity_id))
            .map(|e| e.remove(&component.get_id()));

        let scene_entry = self.interpolated.entry(scene_id.clone()).or_insert(Scene::new());
        let entity_entry = scene_entry.entry(entity_id.clone()).or_insert(Entity::new());
        entity_entry.insert(component.get_id(), component);
    }

    fn remove_component(&mut self, scene_id: &Id, entity_id: &Id, component_id: &Id) {
        self.interpolated
            .get_mut(scene_id)
            .and_then(|s| s.get_mut(entity_id))
            .map(|e| e.remove(component_id));

        let scene_entry = self.removed_components.entry(scene_id.clone()).or_insert(HashMap::new());
        let entity_entry = scene_entry.entry(entity_id.clone()).or_insert(HashSet::new());
        entity_entry.insert(component_id.clone());
    }

    fn interpolate_component_event(&mut self, scene_id: &Id, entity_id: &Id, component_id: &Id, event: Box<dyn Event>, interpolation: f32) {
        let clamped_interpolation = interpolation.min(1.0).max(0.0);

        if let Some(component) = self.get_component(scene_id, entity_id, component_id) {
            let interpolated_component = component.interpolate_event(
                self,
                event,
                clamped_interpolation,
            );

            self.add_component(scene_id, entity_id, interpolated_component);
        } else {
            log!(
                ERROR,
                "Trying to interpolate an event on a component that doesn't exist: ({:?}, {:?}, {:?}) {:?}",
                scene_id,
                entity_id,
                component_id,
                event,
            );
        }
    }

    pub fn get_time(&self) -> Time {
        self.time
    }

    pub fn get_scene(&self, scene_id: &Id) -> Option<&Scene> {
        self.interpolated.get(scene_id).or(self.base.scenes.get(scene_id)).filter(|_| {
            !self.removed_scenes.contains(scene_id)
        })
    }

    pub fn get_entity(&self, scene_id: &Id, entity_id: &Id) -> Option<&Entity> {
        self.get_scene(scene_id).and_then(|s| s.get(entity_id)).filter(|_| {
            !self.removed_entities
                .get(scene_id)
                .map(|s| s.contains(entity_id))
                .unwrap_or(false)
        })
    }

    pub fn get_component(&self, scene_id: &Id, entity_id: &Id, component_id: &Id) -> Option<&Box<dyn Component>> {
        self.get_entity(scene_id, entity_id).and_then(|e| e.get(component_id)).filter(|_| {
            !self.removed_components
                .get(scene_id)
                .and_then(|s| s.get(entity_id))
                .map(|e| e.contains(component_id))
                .unwrap_or(false)
        })
    }
}

pub type Entity = HashMap<Id, Box<dyn Component>>;
pub type Scene = HashMap<Id, Entity>;

#[derive(Debug)]
pub struct SceneStack {
    scenes: HashMap<Id, Scene>,
    scene_order: Vec<Id>,
}

impl SceneStack {
    fn new() -> Self {
        Self {
            scenes: HashMap::new(),
            scene_order: Vec::new(),
        }
    }
}

mod event_thread;