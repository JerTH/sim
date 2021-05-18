/// World

use crate::{collections::{UnsafeAnyExt, Get, GetMut, SparseSet}, components::{ComponentSet, ComponentSetId}, conflictgraph::{ConflictGraph, ConflictGraphError}, debug::*, identity::{EntityId, InternalTypeId, LinearId, SystemExecutionId}, query::{Query}, systems::{DependencyType, SystemId, WorldSystemError, WorldSystem}};

use std::{any::Any, cell::{Cell, RefCell, UnsafeCell}, error::Error, fmt::{Debug, Display}, sync::{Arc, Mutex, MutexGuard, RwLock, RwLockReadGuard}, usize};

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
//
//
//

#[derive(Debug, Clone)]
pub enum WorldCommand {
    AddComponentToEntity(ComponentSetId, EntityId),
    RemoveComponentFromEntity(ComponentSetId, EntityId),
    CreateEntity,
    ResolveSystemTree,
    Stop,
}

/// World
/// 
/// A world encapsulates all objects and possible interactions between objects within it.
/// Everything in a world is an entity, and every entity is described by a set of components.
/// Rules governing entities are described as systems, which operate on sets of components
/// collected by queries.
#[derive(Debug)]
pub struct World {
    // Maybe worth having multiple disjoint "worlds" for entities that never interact
    // Entities in separate worlds could then interact via messages passed between worlds
    // Could be useful for native networking
    // One example of disjoint worlds would be a UI and a 3D environment,
    // or an overworld and a level

    entities: RefCell<Vec<EntityId>>,
    components: SparseSet<UnsafeCell<ComponentSet>, ComponentSetId>, // Using Mutex for now, worth revisiting later
    command_queue: RefCell<Vec<WorldCommand>>,
    
    systems: SparseSet<WorldSystem, SystemId>,
    system_tree: RefCell<Vec<Vec<SystemId>>>,

    // spatial data
}

impl World {
    pub fn new() -> Self {
        World {
            entities: RefCell::new(Vec::new()),
            components: SparseSet::new(),
            command_queue: RefCell::new(Vec::new()),
            systems: SparseSet::new(),
            system_tree: RefCell::new(Vec::new()),
        }
    }

    pub fn new_entity(&self) -> EntityId {
        debug!("creating new entity");
        todo!()
    }

    pub fn add_component<T: Debug + 'static>(&self, entity: EntityId, component: T) {
        //let component_set_id = ComponentSetId::of::<T>();
        //let mut components = self.components.lock();        
        //
        //if let Some(component_set) = components.get_mut(component_set_id) {
        //    component_set.add_component(entity, component);
        //} else {
        //    let component_set = ComponentSet::new::<T>();
        //    //let component_set = ComponentSet {
        //    //    ident: component_set_id,
        //    //    name: String::from("UNIMPLEMENTED"),
        //    //    count: 0,
        //    //    set: Box::new(SparseSet::<T>::new()),
        //    //};
        //    
        //    components.insert_with(component_set_id, component_set);
        //    drop(components); // explicitely drop the RefMut
        //    self.add_component(entity, component); // recursively add component to the new set
        //}

        todo!() // this should probably remain a deferred end-of-frame action using the command interface
    }

    fn resolve_system_tree(&self) -> Result<(), WorldSystemError> {
        let mut graph = ConflictGraph::new();

        for system in self.systems.iter() {
            match graph.insert(system) {
                Ok(()) => {
                    // do nothing for now
                },
                Err(e) => {
                    return Err(WorldSystemError::FailedToResolveSystemTree(e));
                }
            }
        }
        
        let tree = graph.cliques(); // map error to SystemError

        match tree {
            Ok(tree) => {
                *self.system_tree.borrow_mut() = tree.into_iter()
                    .map(|group| group.into_iter()
                    .map(|system| system.id())
                    .collect())
                    .collect();
                return Ok(())
            },
            Err(e) => {
                return Err(WorldSystemError::FailedToResolveSystemTree(e));
            }
        }
    }
    
    // TODO: Probably worth kicking this into its own thread as well, leverage jobs to get rid of "main" thread bottleneck
    pub fn run(&self) {
        'main: loop {
            for system_group in self.system_tree.borrow().iter() {
                // in each iteration of this loop we simulataneously kick off every system of the group
                // each group is internally conflict-free, and so each system in a group can run in parallel

                // TODO: Give systems some ability to multi-thread within themselves, useful for large systems

                // TODO: Give threads some housekeeping tasks when they are idle, self-tests, diagnostics,
                //       sorting component sets, etc. Housekeeping jobs can be "posted" from other places
                //       in the code, e.g. SortComponentRelationship(comp_a, comp_b) and whenever a thread
                //       has time it can take the job and execute it
                
                for system_id in system_group {
                    let system = self.systems.get(system_id);

                    match system {
                        Some(system) => {
                            // queue the system for a thread to pick up
                        },
                        None => {
                            error!("system doesn't exist, system_id={:?}", system_id);
                        }
                    }
                }
            }

            //
            //// Update each system linearly for now. In the future use a dependency graph to automatically parallelize them
            //for system_id in self.system_ids.iter() { // instead of iterating the linear array of system id's, here we will use the dependency graph
            //    //println!("Running system {}\n{:#?}", i, self.systems);
            //    if let Some(system) = self.systems.get(system_id) {
            //        let local_world = LocalWorld {
            //            world: &self,
            //            execution_id: Cell::new(system.execution_id()),
            //        };
            //        
            //        system.run(local_world).expect(format!("Error when running system: {:?}", system).as_str());
            //    } else {
            //        error!("Failed to get system (id: {:?}) from system set", system_id);
            //    }
            //}
            
            // TODO: Probably a good idea to limit the # of commands processed each frame and use a priorty queue of some sort
            for command in self.command_queue.borrow_mut().drain(0..) {
                match command {
                    WorldCommand::AddComponentToEntity(component, entity) => {
                        todo!()
                    },
                    WorldCommand::RemoveComponentFromEntity(component, entity) => {
                        debug!("adding component to entity {}", entity);
                        todo!()
                    },
                    WorldCommand::CreateEntity => {
                        todo!()
                        // can this work? we need to return the entities ID somehow
                        // could reserve the ID and return it at the call-site
                        // maybe 
                    },
                    WorldCommand::ResolveSystemTree => {
                        self.resolve_system_tree();
                    },
                    WorldCommand::Stop => {
                        log!("stopping world simulation");
                        break 'main;
                    },
                }
            }
        }
    }
    
    pub fn add_system(&mut self, system: WorldSystem) -> Result<SystemId, WorldSystemError> {
        match self.systems.insert(system) {
            Ok(id) => {
                // insert succeeded
                if let Some(system) = self.systems.get_mut(id) {
                    // set the systems id to whatever
                    system.set_id(SystemId::from(id));
                } else {
                    // this really shouldn't happen, something bad caused this
                    fatal!("failed to get world system immediately after adding it");
                }
                return Ok(id)
            },
            Err(system) => {
                return Err(WorldSystemError::FailedToAddWorldSystem(system))
            }
        }
    }
    
    fn mark_dependency(&self, dependency: DependencyType, system_id: SystemId, component_id: ComponentSetId) {
        debug!("marking new dependeny: system {:?} depends on component {:?}: {:?}", system_id, component_id, dependency);
        
        todo!("implement me");
        // determine if this new dependency invalidates the work on the current frame
        // there are three options:
        //  1. scrap the entire current frame, rebuild the system tree, and start over
        //  2. cancel this system only, let the frame finish, rebuild the system tree, and continue on the next frame
        //  3. don't scrap anything, proceed as normal, fall-back on a Mutex to protect memory (may lock up)
    }

    pub fn queue_command(&self, command: WorldCommand) {
        self.command_queue.borrow_mut().push(command);
        
        todo!("do something better here, probably get rid of the refcell around command_queue");
    }

    #[allow(dead_code)]
    #[cfg(test)]
    pub(crate) fn test_dummy() -> Self {
        Self::new()
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
    system_id: SystemId,
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
    
    pub(crate) unsafe fn get_component_set(&self, component_set_id: ComponentSetId) -> Option<*mut ComponentSet> {
        match self.world.components.get(component_set_id) {
            Some(cell) => {
                return Some(cell.get());
            },
            None => {
                debug!("couldn't get component set {} from sparse set while attempting to lock it", component_set_id);
                return None;
            }
        }
    }

    pub(crate) fn system_execution_id(&self) -> SystemExecutionId {
        self.execution_id.get()
    }

    pub(crate) fn cached_query_set(&self) -> std::collections::HashMap<SystemExecutionId, Query> {
        todo!()
    }

    pub(crate) fn mark_dependency(&self, dependency: DependencyType, component_id: ComponentSetId) {
        self.world.mark_dependency(dependency, self.system_id, component_id);
    }
    
    //#[deprecated]
    //pub(crate) fn mark_read_dependency<T: Component>(&self) {
    //    if let Some(system) = self.world.systems.as_slice().iter().find(|system| {
    //        system.execution_id() == self.system_execution_id()
    //    }) {
    //        //system.mark_read_dependency::<T>(); // TODO: HANDLE THIS WITH SOMETHING BETTER THAN REFCELL
    //    } else {
    //        fatal!("unable to find system data while attempting to mark read dependency");
    //    }
    //}
    
    //#[deprecated]
    //pub(crate) fn mark_write_dependency<T: Component>(&self) {
    //    if let Some(system) = self.world.systems.as_slice().iter().find(|system| {
    //        system.execution_id() == self.system_execution_id()
    //    }) {
    //        //system.mark_write_dependency::<T>();
    //    } else {
    //        fatal!("unable to find system data while attempting to mark write dependency");
    //    }
    //}
}

pub trait IntoCoordinate {
    fn as_coordinate(&self) -> (f64, f64, f64);
}

impl IntoCoordinate for (f64, f64, f64) {
    fn as_coordinate(&self) -> (f64, f64, f64) {
        *self
    }
}

#[allow(unused_macros)]
macro_rules! query_results {
    ($query:expr, ($($comp:ident),*)) => {
        unsafe { IntoQueryIter::<($($comp),*)>::into_iter(&$query) }
    };
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::IntoQueryIter;

    #[test]
    fn hello_world() {
        let mut world = World::new();

        fn hello_system(_: LocalWorld) -> Result<(), WorldSystemError> {
            debug!("Hello from hello_system!");
            Ok(())
        }

        fn goodbye_system(local_world: LocalWorld) -> Result<(), WorldSystemError> {
            debug!("Goodbye from goodbye_system!");
            local_world.queue_command(WorldCommand::Stop);
            Ok(())
        }

        //world.add_system(&hello_system);
        //world.add_system(&goodbye_system);
        //world.run();
    }

    #[test]
    fn add_components() {
        let mut world = World::new();

        fn entity_producer_system(local_world: LocalWorld) -> Result<(), WorldSystemError> {
            for _ in 0..10 {
                let entity = local_world.new_entity();
                let component = 0usize;
                local_world.add_component(entity, component);
            }
            Ok(())
        }

        fn float_producer_system(local_world: LocalWorld) -> Result<(), WorldSystemError> {
            for entity in local_world.entities() {
                let component = 0f32;
                local_world.add_component(entity, component);
            }
            local_world.queue_command(WorldCommand::Stop);
            Ok(())
        }

        //world.add_system(&entity_producer_system);
        //world.add_system(&float_producer_system);
        //world.run();
    }
    
    #[test]
    fn queries() {
        log!("");
        let mut world = World::new();

        fn entity_producer_system(local_world: LocalWorld) -> Result<(), WorldSystemError> {
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
        
        fn component_query_system(local_world: LocalWorld) -> Result<(), WorldSystemError> {
            debug!("Running component query system");

            for _ in 0..1 {
                let query = Query::new()
                    .with::<f32>()
                    .with::<i32>()
                    .with::<bool>()
                    .make(&local_world);

                for (f, mut b) in query_results!(query, (f32, bool)) {
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

        
        //world.add_system(&entity_producer_system);
        //world.add_system(&component_query_system);
        //world.run();

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

        let world = LocalWorld{
            system_id: SystemId::from(0),
            world: &dummy_world(),
            execution_id: Cell::new(SystemExecutionId::unique()),
        };
        
        let _query = builder.make(&world);
    }
}
