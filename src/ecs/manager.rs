use std::collections::HashMap;
use crate::ecs::entities::{Entity, EntityId};

pub struct EntityManager {
    entities: HashMap<EntityId, Entity>,
    next_id: EntityId,
}

impl EntityManager {
    pub fn new() -> Self {
        EntityManager {
            entities: HashMap::new(),
            next_id: 0,
        }
    }

    pub fn create_entity(&mut self) -> EntityId {
        let id = self.next_id;
        self.next_id += 1;
        self.entities.insert(id, Entity::new(id));
        id
    }

    pub fn get_entity(&self, id: EntityId) -> Option<&Entity> {
        self.entities.get(&id)
    }

    pub fn get_entity_mut(&mut self, id: EntityId) -> Option<&mut Entity> {
        self.entities.get_mut(&id)
    }

    pub fn remove_entity(&mut self, id: EntityId) -> Option<Entity> {
        self.entities.remove(&id)
    }

    pub fn has_entity(&self, id: EntityId) -> bool {
        self.entities.contains_key(&id)
    }

    pub fn with_components<T: 'static>(&self, required: &[std::any::TypeId]) -> Vec<&Entity> {
        self.entities.values().filter(|e| {
            required.iter().all(|tid| e.components.contains_key(tid))
        }).collect()
    }

    pub fn with_components_mut<T: 'static>(&mut self, required: &[std::any::TypeId]) -> Vec<&mut Entity> {
        self.entities.values_mut().filter(|e| {
            required.iter().all(|tid| e.components.contains_key(tid))
        }).collect()
    }

    pub fn all_entities(&self) -> Vec<&Entity> {
        self.entities.values().collect()
    }

    pub fn all_entities_mut(&mut self) -> Vec<&mut Entity> {
        self.entities.values_mut().collect()
    }

    pub fn len(&self) -> usize {
        self.entities.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entities.is_empty()
    }
}
