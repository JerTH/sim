/// World

use crate::{
    collections::{Get, GetMut, SparseSet},
    identity::{EntityId, InternalTypeId, LinearId, LocalExecutionId, SystemId},
    query::{Query, QueryIter, IntoQueryIter}
};

use std::{
    cell::{Cell, RefCell},
    fmt::Debug,
    ops::Deref, 
    usize
};

// Main -> World -> LocalWorld -> WorldSystem -> QueryBuilder -> Query -> QueryResult -> QueryIter -> (Ref/Mut)
//                                    ^                                                                   |
//                                    + - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - +

#[derive(Debug, Clone)]
pub enum WorldCommand {
    Stop,
}

/// World
#[derive(Debug)]
pub struct World {
    entities: RefCell<Vec<EntityId>>,
    components: RefCell<SparseSet<ComponentSet>>,
    systems: SparseSet<WorldSystem>,
    system_ids: Vec<SystemId>,
    command_queue: RefCell<Vec<WorldCommand>>,
    // entity list
    // dependency graph
    // spatial data
}

impl World {
    pub fn new() -> Self {
        World {
            entities: RefCell::new(Vec::new()),
            components: RefCell::new(SparseSet::new()),
            systems: SparseSet::new(),
            system_ids: Vec::new(),
            command_queue: RefCell::new(Vec::new()),
        }
    }

    pub fn new_entity(&self) -> EntityId {
        let entity_id = EntityId::unique();
        self.entities.borrow_mut().push(entity_id);
        entity_id
    }

    pub fn add_component<T: 'static>(&self, entity: EntityId, component: T) {
        let component_set_id = ComponentSetId::of::<T>();
        let mut components = self.components.borrow_mut();
        if let Some(component_set) = components.get_mut(component_set_id) {
            component_set.add_component(entity, component);
        } else {
            let component_set = ComponentSet::new::<T>();
            //let component_set = ComponentSet {
            //    ident: component_set_id,
            //    name: String::from("UNIMPLEMENTED"),
            //    count: 0,
            //    set: Box::new(SparseSet::<T>::new()),
            //};

            components.insert(component_set_id.as_linear_raw() as usize, component_set);
            drop(components); // explicitely drop the RefMut
            self.add_component(entity, component); // recursively add component to the new set
        }
    }
    
    pub fn run(&self) {
        loop {
            // Update each system linearly for now. In the future use a dependency graph to automatically parallelize them
            for system_id in &self.system_ids { // instead of iterating the linear array of system id's, here we will use the dependency graph
                if let Some(system) = self.systems.get(system_id) {
                    let local_world = LocalWorld {
                        world: &self,
                        execution_id: Cell::new(LocalExecutionId::unique()),
                    };
                    
                    system.run(local_world).expect(format!("Error when running system: {:?}", system).as_str());
                }
            }
            

            for command in self.command_queue.borrow_mut().drain(0..) {
                match command {
                    WorldCommand::Stop => { println!("WorldCommand::Stop"); return },
                }
            }
        }
    }
    
    pub fn add_system(&mut self, system: &'static dyn Fn(LocalWorld) -> SystemResult) {
        let system_id = SystemId::unique();
        self.system_ids.push(system_id);
        self.systems.insert(system_id.as_linear_raw() as usize, WorldSystem {
            func: Box::new(system),
            reads: None,
            writes: None,
            name: String::from("UNKNOWN"),
        });
    }

    pub fn queue_command(&self, command: WorldCommand) {
        self.command_queue.borrow_mut().push(command);
    }

    #[allow(dead_code)]
    #[cfg(test)]
    pub(crate) fn test_dummy() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod test_world {
    use super::*;

    #[test]
    fn test_adding_systems() {
        let mut world = World::new();

        fn hello_system(local_world: LocalWorld) -> SystemResult {
            println!("Hello from hello_system!");
            Ok(())
        }

        fn goodbye_system(local_world: LocalWorld) -> SystemResult {
            println!("Goodbye from goodbye_system!");
            local_world.queue_command(WorldCommand::Stop);
            Ok(())
        }

        world.add_system(&hello_system);
        world.add_system(&goodbye_system);
        world.run();
    }

    #[test]
    fn test_create_entities() {
        let mut world = World::new();

        fn entity_producer(local_world: LocalWorld) -> SystemResult {
            let entity = local_world.new_entity();
            Ok(())
        }
    }

    #[test]
    fn test_add_components() {
        let mut world = World::new();

        fn entity_producer_system(local_world: LocalWorld) -> SystemResult {
            for _ in 0..10 {
                let entity = local_world.new_entity();
                let component = 0usize;
                local_world.add_component(entity, component);
            }
            Ok(())
        }

        fn float_producer_system(local_world: LocalWorld) -> SystemResult {
            for entity in local_world.entities() {
                let component = 0f32;
                local_world.add_component(entity, component);
            }
            local_world.queue_command(WorldCommand::Stop);
            Ok(())
        }

        world.add_system(&entity_producer_system);
        world.add_system(&float_producer_system);
        world.run();
    }
    
    #[test]
    fn test_query() {
        let mut world = World::new();

        fn entity_producer_system(local_world: LocalWorld) -> SystemResult {
            for i in 0..77 {
                if i % 3 == 0 {
                    let entity = local_world.new_entity();
                    let float_component = i as f32 * 33.333;
                    let bool_component = i % 7 == 0;
                    local_world.add_component(entity, float_component);
                    local_world.add_component(entity, bool_component);
                } else {
                    let entity = local_world.new_entity();
                    let int_component = i * 10;
                    let bool_component = i % 13 == 0;
                    local_world.add_component(entity, int_component);
                    local_world.add_component(entity, bool_component);
                }
            }
            Ok(())
        }
        
        fn component_query_system(local_world: LocalWorld) -> SystemResult {
            type Component = f32;

            for _ in 0..10000 {
                let query = Query::new()
                    .read::<f32>()
                    .read::<i32>()
                    .read::<bool>()
                    .make(&local_world);

                let result = query.execute();

                let iter = <dyn IntoQueryIter<(Component,)>>::iter(&result);
                let v: Vec<(crate::query::Ref<Component>,)> = iter.collect();
                let l = v.len();
            }

            local_world.queue_command(WorldCommand::Stop);
            Ok(())
        }
        
        world.add_system(&entity_producer_system);
        world.add_system(&component_query_system);
        world.run();
    }
}

type SystemResult = Result<(), ()>;

struct WorldSystem {
    func: Box<dyn Fn(LocalWorld) -> SystemResult>,
    reads: Option<Vec<ComponentSetId>>,
    writes: Option<Vec<ComponentSetId>>,
    name: String,
}

impl WorldSystem {
    fn run(&self, local_world: LocalWorld) -> SystemResult {
        (self.func)(local_world)
    }
}

impl Get<SystemId> for SparseSet<WorldSystem> {
    type Item = WorldSystem;
    fn get(&self, idx: SystemId) -> Option<&Self::Item> {
        self.get(idx.as_linear_raw() as usize)
    }
}

impl GetMut<SystemId> for SparseSet<WorldSystem> {
    type Item = WorldSystem;
    fn get_mut(&mut self, idx: SystemId) -> Option<&mut Self::Item> {
        self.get_mut(idx.as_linear_raw() as usize)
    }
}

impl Get<&SystemId> for SparseSet<WorldSystem> {
    type Item = WorldSystem;
    fn get(&self, idx: &SystemId) -> Option<&Self::Item> {
        self.get(idx.as_linear_raw() as usize)
    }
}

impl GetMut<&SystemId> for SparseSet<WorldSystem> {
    type Item = WorldSystem;
    fn get_mut(&mut self, idx: &SystemId) -> Option<&mut Self::Item> {
        self.get_mut(idx.as_linear_raw() as usize)
    }
}

impl Get<ComponentSetId> for SparseSet<ComponentSet> {
    type Item = ComponentSet;
    fn get(&self, idx: ComponentSetId) -> Option<&Self::Item> {
        self.get(idx.as_linear_raw() as usize)
    }
}

impl GetMut<ComponentSetId> for SparseSet<ComponentSet> {
    type Item = ComponentSet;
    fn get_mut(&mut self, idx: ComponentSetId) -> Option<&mut Self::Item> {
        self.get_mut(idx.as_linear_raw() as usize)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ComponentTypeId(InternalTypeId);

impl ComponentTypeId {
    pub(crate) fn of<T>() -> Self where T: 'static {
        Self(InternalTypeId::of::<T>())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ComponentSetId(InternalTypeId);

impl ComponentSetId {
    pub(crate) fn of<T>() -> Self where T: 'static {
        ComponentSetId(InternalTypeId::of::<SparseSet<T>>())
    }
}

impl LinearId for ComponentSetId {
    fn unique() -> Self {
        ComponentSetId(InternalTypeId::unique())
    }

    fn as_linear_raw(&self) -> u64 {
        self.0.as_linear_raw()
    }
}

#[derive(Debug)]
pub struct ComponentSet {
    ident: ComponentSetId,
    count: usize,
    name: String,
    set: Box<dyn core::any::Any>,
}

impl ComponentSet {
    fn new<T>() -> Self where T: 'static {
        ComponentSet {
            ident: ComponentSetId::of::<T>(),
            count: 0,
            name: String::from(core::any::type_name::<T>()),
            set: Box::new(SparseSet::<T>::new()),
        }
    }

    fn add_component<T: 'static>(&mut self, entity: EntityId, component: T) {
        if self.set.is::<SparseSet<T>>() {
            if let Some(set) = self.set.downcast_mut::<SparseSet<T>>() {
                let result = set.insert(entity.as_linear_raw() as usize, component);
                
                assert!(result.is_none());
                self.count += 1;
            } else {
                panic!("ComponentSet::set downcast failed");
            }
        } else {
            panic!("ComponentSet::set is not SparseSet<T>");
        }
    }

    pub fn contains<T>(&self) -> bool where T: 'static {
        self.ident == ComponentSetId::of::<T>()
    }
    
    pub(crate) fn raw_set<T: 'static>(&self) -> Option<&SparseSet<T>> {
        self.set.downcast_ref::<SparseSet<T>>()
    }

    pub(crate) fn raw_set_mut<T: 'static>(&mut self) -> Option<&mut SparseSet<T>> {
        self.set.downcast_mut::<SparseSet<T>>()
    }
}

pub struct EntityIdIter<'a> {
    entities: &'a RefCell<Vec<EntityId>>,
    idx: usize,
}

impl<'a> Iterator for EntityIdIter<'a> {
    type Item = EntityId;

    fn next(&mut self) -> Option<Self::Item> {
        let entities = self.entities.borrow();
        self.idx += 1;
        entities.get(self.idx - 1).map(|x| *x)
    }
}

#[derive(Debug)]
pub struct LocalWorld<'a> {
    world: &'a World, 
    execution_id: Cell<LocalExecutionId>,
}

impl<'a> LocalWorld<'a> {
    pub fn new_entity(&self) -> EntityId {
        self.world.new_entity()
    }

    pub fn add_component<T: 'static>(&self, entity: EntityId, component: T) {
        self.world.add_component::<T>(entity, component)
    }

    pub fn queue_command(&self, command: WorldCommand) {
        self.world.queue_command(command);
    }

    pub fn entities(&self) -> EntityIdIter {
        EntityIdIter {
            entities: &self.world.entities,
            idx: 0
        }
    }

    pub fn component_set_from_id(&self, id: ComponentSetId) -> Option<&ComponentSet> {
        let set = self.world.components.as_ptr();
        unsafe { (*set).get(id) } // TODO: This is not good, clean this up when adding concurrency
    }

    pub(crate) fn local_execution_id(&self) -> LocalExecutionId {
        self.execution_id.get()
    }

    pub(crate) fn cached_query_set(&self) -> std::collections::HashMap<LocalExecutionId, Query> {
        unimplemented!()
    }
    
    pub(crate) fn mark_component_change(&mut self) {
        unimplemented!()
    }

    pub(crate) fn mark_read_dependency(&mut self, set_id: ComponentSetId) {
        unimplemented!()
    }

    pub(crate) fn mark_write_dependency(&mut self, set_id: ComponentSetId) {
        unimplemented!()
    }

    #[allow(dead_code)]
    #[cfg(test)]
    pub(crate) fn test_dummy(world: &'a World) -> Self {
        LocalWorld {
            world: world,
            execution_id: Cell::new(LocalExecutionId::unique()),
        }
    }
}







// fn system_api(world: &LocalWorld) {
//     // query components
//     // catch and emit events
// }

#[cfg(test)]
mod test {
    use super::*;
    use crate::query::*;

    fn dummy_world<'a>() -> World {
        World::new()
    }

    #[test]
    fn test_queryfilter_sort() {
        let position = (1.0, 2.0, 3.0);
        struct A;
        struct B;
        struct C<T>(T);

        let builder = Query::new() // short circuits if the query was previously constructed and executed
            .read::<A>()
            .read::<C<usize>>()
            .write::<B>()
            .not::<C<A>>()
            .not::<C<B>>()
            .read::<C<C<B>>>()
            .closer_than(10.0, &position)
            .further_than(1.0, &position)
            .sort_filters();

        let world = LocalWorld{ world: &dummy_world(), execution_id: Cell::new(LocalExecutionId::unique()) };
        
        let query = builder.make(&world);

        let _result = query.execute();
    }
}


pub trait Spatial {
    fn position(&self) -> (f64, f64, f64);
}

impl Spatial for (f64, f64, f64) {
    fn position(&self) -> (f64, f64, f64) {
        *self
    }
}


// Debug implementations

impl core::fmt::Debug for WorldSystem {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("WorldSystem")
        .field("func", &"dyn Fn(&LocalWorld) -> SystemResult")
        .field("reads", &self.reads)
        .field("writes", &self.writes)
        .field("name", &self.name)
        .finish()
    }
}
