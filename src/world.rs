/// World

use crate::{collections::{UnsafeAnyExt, Get, GetMut, SparseSet}, debug::*, conflictgraph::ConflictCmp, identity::{EntityId, InternalTypeId, LinearId, SystemExecutionId, SystemId}, query::{Query}};

use std::{any::Any, cell::{Cell, RefCell, UnsafeCell}, fmt::Debug, usize};

//
//
//
// TODO (April 18th)
//
// - Ref<T> functionality
// - Multithreading of systems
// - Systems dependency graph
// - Fallback Mutex guards for safety when depgraph invalid
// - System events
// - Spatial structure
// - Refactor and cleanup `World`
// - Cached query
//
//
//


pub trait Component: Debug + Any + 'static {} 
impl<T> Component for T where T: Debug + Any + 'static {}

#[derive(Debug, Clone)]
pub enum WorldCommand {
    Stop,
    ResolveFlowGraph,
}

/// World
#[derive(Debug)]
pub struct World {
    entities: RefCell<Vec<EntityId>>,
    components: RefCell<SparseSet<ComponentSet>>,
    systems: SparseSet<WorldSystem>,
    system_ids: Vec<SystemId>,
    command_queue: RefCell<Vec<WorldCommand>>,
    // flow graph
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

    pub fn add_component<T: Debug + 'static>(&self, entity: EntityId, component: T) {
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

            components.insert_with(component_set_id.as_linear_raw() as usize, component_set);
            drop(components); // explicitely drop the RefMut
            self.add_component(entity, component); // recursively add component to the new set
        }
    }
    
    pub fn run(&self) {

        // when running systems in parallel, maybe wrap each system in a go/finished block that chains condvars based on data dependency? 
        
        loop {
            // Update each system linearly for now. In the future use a dependency graph to automatically parallelize them
            for system_id in self.system_ids.iter() { // instead of iterating the linear array of system id's, here we will use the dependency graph
                //println!("Running system {}\n{:#?}", i, self.systems);
                if let Some(system) = self.systems.get(system_id) {
                    let local_world = LocalWorld {
                        world: &self,
                        execution_id: Cell::new(system.execution_id()),
                    };
                    
                    system.run(local_world).expect(format!("Error when running system: {:?}", system).as_str());
                } else {
                    error!("Failed to get system (id: {:?}) from system set", system_id);
                }
            }

            for command in self.command_queue.borrow_mut().drain(0..) {
                match command {
                    WorldCommand::Stop => { debug!("WorldCommand::Stop"); return },
                    WorldCommand::ResolveFlowGraph => {
                        // Collect all read-only systems

                        //let read_only: Vec<&WorldSystem> = self.systems.as_slice().iter().filter(|system| system.writes.borrow().is_empty()).collect();
                        //let writes: Vec<&WorldSystem> = self.systems.as_slice().iter().filter(|system| !system.writes.borrow().is_empty()).collect();

                        // for each system
                        //   for each dep of that system
                        //     for each other system
                        //       for each dep of the other system
                        //       
                    }
                }
            }
        }
    }
    
    pub fn add_system(&mut self, system: &'static dyn Fn(LocalWorld) -> SystemResult) {
        let system_id = SystemId::unique();
        self.system_ids.push(system_id);
        self.systems.insert_with(system_id.as_linear_raw() as usize, WorldSystem {
            func: Box::new(system),
            reads: RefCell::new(Vec::new()),
            writes: RefCell::new(Vec::new()),
            exec_id: SystemExecutionId::unique(),
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


type SystemResult = Result<(), ()>;

struct WorldSystem {
    exec_id: SystemExecutionId,
    func: Box<dyn Fn(LocalWorld) -> SystemResult>,
    reads: RefCell<Vec<ComponentSetId>>,
    writes: RefCell<Vec<ComponentSetId>>,
    name: String,
}

impl WorldSystem {
    fn run(&self, local_world: LocalWorld) -> SystemResult {
        (self.func)(local_world)
    }

    fn execution_id(&self) -> SystemExecutionId {
        self.exec_id
    }

    pub(crate) fn mark_write_dependency<T>(&self) where T: Component {
        self.writes.borrow_mut().push(ComponentSetId::of::<T>())
    }

    pub(crate) fn mark_read_dependency<T>(&self) where T: Component {
        self.reads.borrow_mut().push(ComponentSetId::of::<T>())
    }
}

impl ConflictCmp for WorldSystem {
    fn conflict_cmp(&self, other: &Self) -> bool {
        // conflict if two systems write the same data or one node reads and the other node writes

        for read in self.reads.borrow().iter() {
            for write in other.writes.borrow().iter() {
                if read == write {
                    return true
                }
            }
        }

        for write in self.writes.borrow().iter() {
            for read in other.reads.borrow().iter() {
                if write == read {
                    return true
                }
            }
            
            for other_write in other.writes.borrow().iter() {
                if write == other_write {
                    return true
                }
            }
        }
        return false
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

type RawComponentSet<T> = SparseSet<UnsafeCell<T>>;

#[derive(Debug)]
pub struct ComponentSet {
    ident: ComponentSetId,
    count: usize,
    name: String,
    set: Box<dyn Any>,
}

impl ComponentSet {
    fn new<T: Debug>() -> Self where T: 'static {
        ComponentSet {
            ident: ComponentSetId::of::<T>(),
            count: 0,
            name: String::from(core::any::type_name::<T>()),
            set: Box::new(RawComponentSet::<T>::new()),
        }
    }
    
    fn add_component<T: Component>(&mut self, entity: EntityId, component: T) {
        if self.set.is::<RawComponentSet<T>>() {
            if let Some(set) = self.set.downcast_mut::<RawComponentSet<T>>() {
                let result = set.insert_with(entity.as_linear_raw() as usize, UnsafeCell::new(component));
                
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

    pub(crate) fn component_set_id(&self) -> ComponentSetId {
        self.ident
    }
    
    pub(crate) fn raw_set<T: 'static>(&self) -> Option<&RawComponentSet<T>> {
        self.set.downcast_ref::<RawComponentSet<T>>()
    }

    pub(crate) unsafe fn raw_set_unchecked<T: 'static>(&self) -> &RawComponentSet<T> {
        self.set.downcast_ref_unchecked::<RawComponentSet<T>>()
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
    execution_id: Cell<SystemExecutionId>,
}

impl<'a> LocalWorld<'a> {
    pub fn new_entity(&self) -> EntityId {
        self.world.new_entity()
    }

    pub fn add_component<T: Debug + 'static>(&self, entity: EntityId, component: T) {
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

    pub(crate) fn system_execution_id(&self) -> SystemExecutionId {
        self.execution_id.get()
    }

    pub(crate) fn cached_query_set(&self) -> std::collections::HashMap<SystemExecutionId, Query> {
        todo!()
    }

    pub(crate) fn mark_read_dependency<T: Component>(&self) {
        if let Some(system) = self.world.systems.as_slice().iter().find(|system| {
            system.execution_id() == self.system_execution_id()
        }) {
            system.mark_read_dependency::<T>();
        } else {
            fatal!("unable to find system data while attempting to mark read dependency");
        }
    }

    pub(crate) fn mark_write_dependency<T: Component>(&self) {
        if let Some(system) = self.world.systems.as_slice().iter().find(|system| {
            system.execution_id() == self.system_execution_id()
        }) {
            system.mark_write_dependency::<T>();
        } else {
            fatal!("unable to find system data while attempting to mark write dependency");
        }
    }

    #[allow(dead_code)]
    #[cfg(test)]
    pub(crate) fn test_dummy(world: &'a World) -> Self {
        LocalWorld {
            world: world,
            execution_id: Cell::new(SystemExecutionId::unique()),
        }
    }
}




pub trait IntoCoordinate {
    fn as_coordinate(&self) -> (f64, f64, f64);
}

impl IntoCoordinate for (f64, f64, f64) {
    fn as_coordinate(&self) -> (f64, f64, f64) {
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



#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::IntoQueryIter;

    #[test]
    fn hello_world() {
        let mut world = World::new();

        fn hello_system(_: LocalWorld) -> SystemResult {
            debug!("Hello from hello_system!");
            Ok(())
        }

        fn goodbye_system(local_world: LocalWorld) -> SystemResult {
            debug!("Goodbye from goodbye_system!");
            local_world.queue_command(WorldCommand::Stop);
            Ok(())
        }

        world.add_system(&hello_system);
        world.add_system(&goodbye_system);
        world.run();
    }

    #[test]
    fn add_components() {
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
    fn queries() {
        log!("");
        let mut world = World::new();

        fn entity_producer_system(local_world: LocalWorld) -> SystemResult {
            debug!("Running entity producer system");

            for i in 0..10 {
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
            debug!("Running component query system");

            for _ in 0..1 {
                let query = Query::new()
                    .with::<f32>()
                    .with::<i32>()
                    .with::<bool>()
                    .make(&local_world);

                for (f, mut b) in IntoQueryIter::<(i32, bool)>::into_iter(&query) {
                    debug!("Got components: {:?}", (*f, *b));
                    *b = !*b;
                }
            }
            
            local_world.queue_command(WorldCommand::Stop);
            Ok(())
        }

        // April 4th 2020
        // 
        // ECS is sort of working. Key things to do now are:
        //  - Clean up the code. Query is especially a mess. May require some better/new abstractions
        //  - Refine the Query -> QueryIter transformation
        //  - Implement the various IntoQueryIter overloads
        //  - Implement Mut/Ref query returns. Right now it only returns Ref
        //  - Transition from single threaded/RefCell impl to multi-threaded impl
        //  
        // Future things to potentially implement:
        //  - Implement WorldSystem diagnostics, dependency graph, auto-parallel, etc
        //  - Add robust MPSC logging
        //  - Implement events for systems through LocalWorld interface. May add another guard on LocalWorld that notifies of events when its taken
        //  - Make dropping LocalWorld after a system finishes running perform some cleanup/diagnostics/event handling
        //  - Investigate SIMD bit comparisons for filtering dead/alive entities or components from control bytes
        //  - Double buffer some or all component state, and intercept reads and writes through Mut to reference the correct state copy
        // 

        
        world.add_system(&entity_producer_system);
        world.add_system(&component_query_system);
        world.run();

        log!("-------------------------------------------------------");
        log!("                   world data dump                     ");
        log!("-------------------------------------------------------");
        log!("{:#?}", world);
        log!("-------------------------------------------------------");
        log!("                 end world data dump                   ");
        log!("-------------------------------------------------------");

    }

    fn dummy_world<'a>() -> World {
        World::new()
    }

    #[test]
    fn filters() {
        let position = (1.0, 2.0, 3.0);
        #[derive(Debug)] struct A;
        #[derive(Debug)] struct B;
        #[derive(Debug)] struct C<T>(T);

        let builder = Query::new() // short circuits if the query was previously constructed and executed
            .with::<A>()
            .with::<C<usize>>()
            .with::<B>()
            .not::<C<A>>()
            .not::<C<B>>()
            .with::<C<C<B>>>()
            .closer_than(10.0, &position)
            .further_than(1.0, &position)
            .sort_filters();

        let world = LocalWorld{ world: &dummy_world(), execution_id: Cell::new(SystemExecutionId::unique()) };
        
        let _query = builder.make(&world);
    }
}
