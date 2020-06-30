#![allow(dead_code)]

use std::any::{Any, TypeId};
use std::collections::{btree_map, BTreeMap, BTreeSet};
use std::sync::atomic::{self, AtomicU64};

#[derive(Copy, Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
#[repr(transparent)]
pub struct Entity(u64);

struct Entities {
    items: BTreeSet<Entity>,
    next_id: AtomicU64,
}

impl Entities {
    pub fn create(&mut self) -> Entity {
        let entity = Entity(self.next_id.fetch_add(1, atomic::Ordering::SeqCst));
        self.items.insert(entity);
        entity
    }

    pub fn destroy(&mut self, entity: Entity) {
        self.items.remove(&entity);
    }
}

pub trait Component: Copy + 'static {}

struct Components {
    types: BTreeMap<TypeId, Box<dyn Any>>,
}

impl Components {
    pub fn map<T: Component>(&self) -> Option<&BTreeMap<Entity, T>> {
        self.types
            .get(&TypeId::of::<T>())
            .map(|t| &t.downcast_ref::<ComponentType<T>>().unwrap().map)
    }

    pub fn map_mut<T: Component>(&mut self) -> &mut BTreeMap<Entity, T> {
        let type_entry = self.types.entry(TypeId::of::<T>()).or_insert_with(|| {
            Box::new(ComponentType::<T> {
                map: BTreeMap::new(),
            })
        });
        &mut type_entry.downcast_mut::<ComponentType<T>>().unwrap().map
    }

    pub fn entry_mut<T: Component>(&mut self, entity: Entity) -> btree_map::Entry<Entity, T> {
        self.map_mut::<T>().entry(entity)
    }

    pub fn insert<T: Component>(&mut self, entity: Entity, component: T) -> Option<T> {
        self.map_mut::<T>().insert(entity, component)
    }

    pub fn remove<T: Component>(&mut self, entity: Entity) -> Option<T> {
        self.types.get_mut(&TypeId::of::<T>()).and_then(|t| {
            t.downcast_mut::<ComponentType<T>>()
                .unwrap()
                .map
                .remove(&entity)
        })
    }
}

struct ComponentType<T> {
    map: BTreeMap<Entity, T>,
}

pub struct ECS {
    entities: Entities,
    components: Components,
    systems: Vec<Box<dyn System>>,
}

pub struct ProcessContext<'a> {
    ecs: &'a mut ECS,
}

impl<'a> ProcessContext<'a> {
    pub fn create_entity(&mut self) -> Entity {
        self.ecs.entities.create()
    }

    pub fn destroy_entity(&mut self, entity: Entity) {
        self.ecs.entities.destroy(entity);
    }

    pub fn add_component<T: Component>(&mut self, entity: Entity, component: T) {
        self.ecs.components.insert(entity, component);
    }
}

pub trait System {
    fn process(&self, context: &mut ProcessContext);
}
