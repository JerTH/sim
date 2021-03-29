/// World


use crate::collections::SparseSet;
use crate::identity::{ InternalTypeId, InstanceId };
use std::{cmp::Ordering, collections::{HashMap, btree_set::Union}, usize};
use std::fmt::Debug; 
use std::cell::Cell;

type SystemResult = Result<(), ()>;

struct WorldSystem {
    func: Box<dyn Fn(&LocalWorld) -> SystemResult>,
    reads: Vec<ComponentSetId>,
    writes: Vec<ComponentSetId>,
    name: String,
}

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct ComponentTypeId(InternalTypeId);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct ComponentContainerTypeId(InternalTypeId);

impl ComponentContainerTypeId {
    fn from_content_type<T>() -> Self where T: 'static {
        ComponentContainerTypeId(InternalTypeId::of::<SparseSet<T>>())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ComponentSetId {
    container_id: ComponentContainerTypeId,
    content_id: ComponentTypeId,
    instance_id: InstanceId,
}

impl ComponentSetId {
    fn new<T>() -> Self where T: 'static {
        ComponentSetId {
            container_id: ComponentContainerTypeId(InternalTypeId::of::<SparseSet<T>>()),
            content_id: ComponentTypeId(InternalTypeId::of::<T>()),
            instance_id: InstanceId::unique(),
        }
    }
}

#[derive(Debug)]
struct ComponentSet {
    count: usize,
    ident: ComponentSetId,
    set: Box<dyn core::any::Any>,
    name: String,
}

impl ComponentSet {
    fn new<T>(name: &str) -> Self where T: 'static {
        ComponentSet {
            count: 0,
            ident: ComponentSetId::new::<T>(),
            set: Box::new(SparseSet::<T>::new()),
            name: String::from(name),
        }
    }

    fn content_id(&self) -> ComponentTypeId {
        self.ident.content_id
    }

    fn instance_id(&self) -> InstanceId {
        self.ident.instance_id
    }

    
}

#[derive(Debug)]
pub struct World {
    components: HashMap<ComponentSetId, ComponentSet>,
    systems: Vec<WorldSystem>,
    // entity list
    // dependency graph
    // spatial data
}

impl World {
    fn component_count(&self, tid: &ComponentContainerTypeId) -> usize {
        unimplemented!()
    }

    fn run_system(&self, system: (usize, impl Fn(&LocalWorld) -> SystemResult)) {
        let local_world = LocalWorld {
            world: self,
            execution_id: core::cell::Cell::new(LocalExecutionId(system.0, 0usize)),
        };

        (system.1)(&local_world).expect("System failed") // Can test access on the first run, use that to schedule future async runs with other systems. Fallback synchronization incase access changes in the system - react to that
    }
}

struct Position;
impl Position {
    fn new() -> Self { unimplemented!() }
}

struct Velocity;
struct Player;
struct Enemy;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
struct LocalExecutionId(usize, usize);
struct LocalWorld<'a> {
    world: &'a World, 
    execution_id: Cell<LocalExecutionId>,
}

impl<'a> LocalWorld<'a> {
    fn local_execution_id(&self) -> LocalExecutionId {
        let id = self.execution_id.get();
        self.execution_id.set(LocalExecutionId(id.0, id.1 + 1));
        return id
    }

    fn cached_query_set(&self) -> std::collections::HashMap<LocalExecutionId, Query> {
        unimplemented!()
    }
    
    fn minimum_component_set(&self, filter_set: FilterSet) -> Option<ComponentContainerTypeId> {
        let mut minimum = 0usize;
        let mut type_id = None;

        let mut compare_component_count = |tid| {
            type_id = if minimum.min(self.world.component_count(tid)) == minimum {
                Some(*tid)
            } else {
                type_id
            }
        };

        for filter in &filter_set {
            match filter {
                QueryFilter::ComponentRead(tid) => (compare_component_count)(&tid), // cache the set order in an ordered vec and update with changes?
                QueryFilter::ComponentWrite(tid) => (compare_component_count)(&tid),
                QueryFilter::ComponentNot(tid) => (compare_component_count)(&tid),
                _ => continue
            }
        }
        type_id
    }
}

#[derive(Debug)]
struct Query {
    read_container_ids: Vec<ComponentContainerTypeId>,
    write_container_ids: Vec<ComponentContainerTypeId>,
}

impl<'a> Query {
    fn new() -> QueryBuilder<'a> {
        QueryBuilder {
            local_world: None,
            filter_set: FilterSet::new(),
            cached: None,
        }
    }

    fn cached(world: &'a LocalWorld) -> QueryBuilder<'a> {
        if world.cached_query_set().contains_key(&world.local_execution_id()) {
            // we've cached this query, fetch and return it
            unimplemented!()
        } else {
            // this is the first time we've seen this query
            QueryBuilder {
                local_world: Some(world),
                filter_set: FilterSet::new(),
                cached: None,
            }
        }
    }

    fn execute(self, world: &LocalWorld) -> QueryResult<'a> {
        // - filter on spatial constraints and collect the list of entities
        // - test for the minimum constraint set, including the constrained spatial set
        // - begin iterating over the minimum set, test each constraint in minimum set order
        // - yield components of entities which satisfy all constraints

        

        unimplemented!()
    }
}

struct QueryResult<'a> {
    component_sets: Vec<&'a ComponentSet>,
}

trait IntoQueryIter<'a, T: core::any::Any> {
    fn iter(&self) -> QueryIter<'a, T>;
}

impl<'a, A: core::any::Any> IntoQueryIter<'a, (A,)> for QueryResult<'a> {
    fn iter(&self) -> QueryIter<'a, (A,)> {
        for set in self.component_sets.iter() {
            if set.content_id() == ComponentTypeId(InternalTypeId::of::<A>()) {
                return QueryIter::<(A,)> {
                    minimum_set: set,
                    phantom: core::marker::PhantomData,
                }
            }
        }

        // lots of things to untangle:
        // - the typeid of the components and of their storage and of their storage wrapped in Mut or Ref are all different
        // - grab the sparsesets from the world in such a way that locks them when mutated
        // - sort the filter set in Query::make()
        // - robustly get and maintain knowledge of the hierarchy of minimum sets in the world
        // - implement query caching
        // - analyze the read/write pattern of each system and parallel run them where possible
        // - make adding component types to the world ridiculously simple, preferrably without having to "register" them at all
        // - double buffer component state, and intercept reads and writes through Mut to reference the correct state copy
        // - investigate SIMD bit comparisons for filtering dead/alive entities or components from control bytes

        panic!("Query result doesn't contain component set")
    }
}

impl<'a, A: core::any::Any, B: core::any::Any> IntoQueryIter<'a, (A, B)> for QueryResult<'a> {
    fn iter(&self) -> QueryIter<'a, (A, B)> {
        unimplemented!()
    }
}

impl<'a, A: core::any::Any, B: core::any::Any, C: core::any::Any> IntoQueryIter<'a, (A, B, C)> for QueryResult<'a> {
    fn iter(&self) -> QueryIter<'a, (A, B, C)> {
        unimplemented!()
    }
}

struct QueryIter<'a, T> {
    minimum_set: &'a ComponentSet,
    phantom: core::marker::PhantomData<T>
}

impl<'a, T> Iterator for QueryIter<'a, T> {
    type Item = T;
    fn next(&mut self) -> Option<Self::Item> {
        unimplemented!()
    }
}

#[derive(Debug, Copy, Clone)]
enum QueryFilter {
    None,
    ComponentRead(ComponentContainerTypeId),
    ComponentWrite(ComponentContainerTypeId),
    ComponentNot(ComponentContainerTypeId),
    SpatialCloserThan(f64, (f64, f64, f64)),
    SpatialFurtherThan(f64, (f64, f64, f64)),
    // SpatialFrustum
}

impl QueryFilter {
    fn precedence(&self) -> usize {
        match self {
            // precendence: lower value is higher precedence
            QueryFilter::None => usize::MAX,
            QueryFilter::SpatialCloserThan(_, _) => 10,
            QueryFilter::SpatialFurtherThan(_, _) => 10,
            QueryFilter::ComponentWrite(_) => 20,
            QueryFilter::ComponentRead(_) => 20,
            QueryFilter::ComponentNot(_) => 30,
        }
    }
}

impl PartialEq for QueryFilter {
    fn eq(&self, other: &Self) -> bool {
        PartialEq::eq(&self.precedence(), &other.precedence())
    }
}

impl Eq for QueryFilter { }

impl PartialOrd for QueryFilter {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        PartialOrd::partial_cmp(&self.precedence(), &other.precedence())
    }
}

impl Ord for QueryFilter {
    fn cmp(&self, other: &Self) -> Ordering {
        Ord::cmp(&self.precedence(), &other.precedence())
    }
}

type FilterSet = Vec<QueryFilter>;

struct QueryBuilder<'a> {
    local_world: Option<&'a LocalWorld<'a>>,
    filter_set: Vec<QueryFilter>,
    cached: Option<Query>,
}

impl<'a> QueryBuilder<'a> {
    fn read<T>(mut self) -> Self where T: 'static {
        if self.cached.is_some() { return self };
        self.filter_set.push(QueryFilter::ComponentRead(ComponentContainerTypeId::from_content_type::<T>()));
        self
    }

    fn write<T>(mut self) -> Self where T: 'static {
        if self.cached.is_some() { return self };
        self.filter_set.push(QueryFilter::ComponentWrite(ComponentContainerTypeId::from_content_type::<T>()));
        self
    }

    fn not<T>(mut self) -> Self where T: 'static {
        if self.cached.is_some() { return self };
        self.filter_set.push(QueryFilter::ComponentNot(ComponentContainerTypeId::from_content_type::<T>()));
        self
    }

    fn closer_than(mut self, max_dist: f64, pos: &impl Spatial) -> Self {
        if self.cached.is_some() { return self };
        let xyz = pos.position();
        self.filter_set.push(QueryFilter::SpatialCloserThan(max_dist, xyz));
        self
    }
    
    fn further_than(mut self, min_dist: f64, pos: &impl Spatial) -> Self {
        if self.cached.is_some() { return self };
        let xyz = pos.position();
        self.filter_set.push(QueryFilter::SpatialFurtherThan(min_dist, xyz));
        self
    }

    fn sort_filters(mut self) -> Self {
        if self.cached.is_some() { return self };
        self.filter_set.sort();
        println!("{:#?}", self.filter_set);
        self
    }

    fn make(self) -> Query {
        if self.cached.is_some() {
            // use the cached query
        };

        let local_world = self.local_world;
        
        let filter_count = self.filter_set.len();

        soft_unimplemented!();
        Query {
            read_container_ids: Vec::new(),
            write_container_ids: Vec::new(),
        }
    }

    fn execute(self, world: &LocalWorld) -> QueryResult<'a> {
        self.make().execute(world)
    }
}

// fn system_api(world: &LocalWorld) {
//     // query components
//     // catch and emit events
// }

#[cfg(test)]
mod test {
    use super::*;

    fn _dummy_local_world<'a>() -> LocalWorld<'a> {
        unimplemented!()
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

        let query = builder.make();
    }
}

struct Mut<'a, T> {
    phantom: core::marker::PhantomData<&'a mut T>,
}

struct Ref<'a, T> {
    phantom: core::marker::PhantomData<&'a T>,
}

trait Spatial {
    fn position(&self) -> (f64, f64, f64);
}

impl Spatial for Position {
    fn position(&self) -> (f64, f64, f64) {
        unimplemented!()
    }
}

impl Spatial for (f64, f64, f64) {
    fn position(&self) -> (f64, f64, f64) {
        *self
    }
}
