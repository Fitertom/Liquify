use std::any::{Any, TypeId};
use std::collections::HashMap;
use crate::ecs::world::World;

pub type EntityId = u32;

pub struct Entity {
    pub id: EntityId,
    pub components: HashMap<TypeId, Box<dyn Any>>,
}

impl Entity {
    pub fn new(id: EntityId) -> Self {
        Entity {
            id,
            components: HashMap::new(),
        }
    }

    pub fn add_component<T: 'static>(&mut self, component: T) {
        self.components.insert(TypeId::of::<T>(), Box::new(component));
    }

    pub fn get_component<T: 'static>(&self) -> Option<&T> {
        self.components.get(&TypeId::of::<T>()).and_then(|b| b.downcast_ref())
    }

    pub fn get_component_mut<T: 'static>(&mut self) -> Option<&mut T> {
        self.components.get_mut(&TypeId::of::<T>()).and_then(|b| b.downcast_mut())
    }

    pub fn has_component<T: 'static>(&self) -> bool {
        self.components.contains_key(&TypeId::of::<T>())
    }

    pub fn remove_component<T: 'static>(&mut self) -> Option<T> {
        self.components.remove(&TypeId::of::<T>())
            .and_then(|b| b.downcast().ok().map(|b| *b))
    }
}

pub type SystemFn = Box<dyn FnMut(&mut World)>;

pub struct SystemRegistry {
    systems: Vec<(String, SystemFn)>,
}

impl SystemRegistry {
    pub fn new() -> Self {
        SystemRegistry {
            systems: Vec::new(),
        }
    }

    pub fn register<F: FnMut(&mut World) + 'static>(&mut self, name: &str, system: F) {
        self.systems.push((name.to_string(), Box::new(system)));
    }

    pub fn run_all(&mut self, world: &mut World) {
        for (_name, system) in self.systems.iter_mut() {
            system(world);
        }
    }

    pub fn run_named(&mut self, world: &mut World, name: &str) -> bool {
        if let Some((_name, system)) = self.systems.iter_mut().find(|(n, _)| n == name) {
            system(world);
            true
        } else {
            false
        }
    }
}
