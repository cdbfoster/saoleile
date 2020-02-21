use std::collections::HashMap;

use crate::component::Component;
use crate::event::{Event, EventReceiver};
use crate::event::component::{AddComponentEvent, ComponentEvent, RemoveComponentEvent};
use crate::util::Id;

pub struct Entity {
    id: Id,
    components: HashMap<Id, Box<dyn Component>>,
}

impl Entity {
    pub fn new(id: Id, components: Vec<Box<dyn Component>>) -> Self {
        Self {
            id,
            components: components.into_iter().map(|c| (c.get_id(), c)).collect(),
        }
    }

    pub fn get_component(&self, id: &Id) -> Result<&dyn Component, String> {
        if let Some(component) = self.components.get(id) {
            Ok(&**component)
        } else {
            Err(format!("Entity::get_component: entity \"{}\" doesn't have component \"{}\"", self.id, id))
        }
    }

    fn add_component(&mut self, component: Box<dyn Component>) -> Result<(), String> {
        let id = component.get_id();

        if self.components.contains_key(&id) {
            return Err(format!("Entity::add_component: entity \"{}\" already has component \"{}\"", self.id, id));
        }

        self.components.insert(id, component);
        Ok(())
    }

    fn remove_component(&mut self, id: &Id) -> Result<(), String> {
        if self.components.remove(id).is_some() {
            Ok(())
        } else {
            Err(format!("Entity::remove_component: entity \"{}\" doesn't have component \"{}\"", self.id, id))
        }
    }

    fn get_component_mut(&mut self, id: &Id) -> Result<&mut dyn Component, String> {
        if let Some(component) = self.components.get_mut(id) {
            Ok(&mut **component)
        } else {
            Err(format!("Entity::get_component_mut: entity \"{}\" doesn't have component \"{}\"", self.id, id))
        }
    }
}

impl EventReceiver for Entity {
    fn process_event(&mut self, event: Box<dyn Event>) {
        if event.as_any().is::<ComponentEvent>() {
            let component_event = *event.as_boxed_any().downcast::<ComponentEvent>().unwrap();

            match self.get_component_mut(&component_event.destination) {
                Ok(component) => component.process_event(component_event.payload),
                Err(message) => log!(ERROR, "entity \"{}\" event error: {}", self.id, message),
            }
        } else if event.as_any().is::<AddComponentEvent>() {
            let add_component_event = *event.as_boxed_any().downcast::<AddComponentEvent>().unwrap();

            match self.add_component(add_component_event.component) {
                Err(message) => log!(ERROR, "entity \"{}\" event error: {}", self.id, message),
                _ => (),
            }
        } else if event.as_any().is::<RemoveComponentEvent>() {
            let remove_component_event = *event.as_boxed_any().downcast::<RemoveComponentEvent>().unwrap();

            match self.remove_component(&remove_component_event.id) {
                Err(message) => log!(ERROR, "entity \"{}\" event error: {}", self.id, message),
                _ => (),
            }
        }
    }
}