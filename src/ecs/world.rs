use crate::ecs::entities::EntityId;
use crate::ecs::manager::EntityManager;
use crate::ecs::entities::Entity;

pub struct World {
    pub entities: EntityManager,
}

impl World {
    pub fn new() -> Self {
        World {
            entities: EntityManager::new(),
        }
    }

    pub fn create_entity(&mut self) -> EntityId {
        self.entities.create_entity()
    }

    pub fn get_entity(&self, id: EntityId) -> Option<&Entity> {
        self.entities.get_entity(id)
    }

    pub fn get_entity_mut(&mut self, id: EntityId) -> Option<&mut Entity> {
        self.entities.get_entity_mut(id)
    }

    pub fn get_component<T: 'static>(&self, id: EntityId) -> Option<&T> {
        self.entities.get_entity(id).and_then(|e| e.get_component::<T>())
    }

    pub fn get_component_mut<T: 'static>(&mut self, id: EntityId) -> Option<&mut T> {
        self.entities.get_entity_mut(id).and_then(|e| e.get_component_mut::<T>())
    }

    pub fn add_component<T: 'static>(&mut self, id: EntityId, component: T) -> bool {
        if let Some(entity) = self.entities.get_entity_mut(id) {
            entity.add_component(component);
            true
        } else {
            false
        }
    }

    pub fn query_with<T: 'static>(&self) -> Vec<(&Entity, &T)> {
        self.entities.all_entities().into_iter().filter_map(|e| {
            e.get_component::<T>().map(|c| (e, c))
        }).collect()
    }

    pub fn query_with_mut<T: 'static>(&mut self) -> Vec<EntityId> {
        self.entities.all_entities()
            .into_iter()
            .filter_map(|e: &Entity| e.get_component::<T>().map(|_| e.id))
            .collect()
    }

    pub fn query_with_two<T: 'static, U: 'static>(&self) -> Vec<(&Entity, &T, &U)> {
        self.entities.all_entities().into_iter().filter_map(|e| {
            if e.has_component::<T>() && e.has_component::<U>() {
                let c1 = e.get_component::<T>().unwrap();
                let c2 = e.get_component::<U>().unwrap();
                Some((e, c1, c2))
            } else {
                None
            }
        }).collect()
    }
}
