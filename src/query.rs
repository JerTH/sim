use std::{any::{Any, TypeId}, cell::UnsafeCell, fmt::Debug, ops::{Deref, DerefMut}};

use crate::{
    identity::{EntityId, LocalExecutionId}, 
    world::{ComponentSet, ComponentSetId, LocalWorld, IntoCoordinate},
    collections::{Get, GetMut},
    debug::*,
};





#[derive(Debug, Copy, Clone)]
pub enum QueryFilter {
    ComponentAnyChanged(ComponentSetId), // rejects all entities if none of their given component has changed
    ComponentChanged(ComponentSetId), // passes only the entities which have had the given component change since the last query in this system
    ComponentAccess(ComponentSetId), // passes all entities with the given component, with read access to that component
    ComponentWrite(ComponentSetId), // passes all entities with the given component, with write access to that component
    ComponentNot(ComponentSetId), // rejects all entities with the given component
    SpatialCloserThan(f64, (f64, f64, f64)), // rejects all entities which can be considered further than the given linear distance from the given 3D point
    SpatialFurtherThan(f64, (f64, f64, f64)), // rejects all entities which can be considered closer than the given linear distance from the given 3D point
    // SpatialFrustum, // rejects all entities which lay outside of the given frustum
}

impl QueryFilter {
    fn precedence(&self) -> usize {
        match self {
            // precendence: lower value is higher precedence
            QueryFilter::ComponentAnyChanged(_) => 10,
            QueryFilter::ComponentChanged(_) => 10,
            QueryFilter::SpatialCloserThan(_, _) => 10,
            QueryFilter::SpatialFurtherThan(_, _) => 10,
            QueryFilter::ComponentNot(_) => 20,
            QueryFilter::ComponentWrite(_) => 1000,
            QueryFilter::ComponentAccess(_) => 1000,
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
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        PartialOrd::partial_cmp(&self.precedence(), &other.precedence())
    }
}

impl Ord for QueryFilter {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        Ord::cmp(&self.precedence(), &other.precedence())
    }
}







pub struct QueryBuilder<'a> {
    filter_set: Vec<QueryFilter>,
    components: Vec<ComponentSetId>,
    cached: Option<(Query<'a>, &'a LocalWorld<'a>)>,
}

impl<'a> QueryBuilder<'a> {
    pub fn with<T>(mut self) -> Self where T: 'static {
        if self.cached.is_some() { return self };

        let id = ComponentSetId::of::<T>();
        self.filter_set.push(QueryFilter::ComponentAccess(id));
        self.components.push(id);
        self
    }
    
    pub fn not<T>(mut self) -> Self where T: 'static {
        if self.cached.is_some() { return self };
        self.filter_set.push(QueryFilter::ComponentNot(ComponentSetId::of::<T>()));
        self
    }

    pub fn changed<T>(mut self) -> Self where T: 'static {
        if self.cached.is_some() { return self };
        self.filter_set.push(QueryFilter::ComponentChanged(ComponentSetId::of::<T>()));
        self
    }

    pub fn any_changed<T>(mut self) -> Self where T: 'static {
        if self.cached.is_some() { return self };
        self.filter_set.push(QueryFilter::ComponentAnyChanged(ComponentSetId::of::<T>()));
        self
    }

    pub fn closer_than(mut self, max_dist: f64, pos: &impl IntoCoordinate) -> Self {
        if self.cached.is_some() { return self };
        let xyz = pos.as_coordinate();
        self.filter_set.push(QueryFilter::SpatialCloserThan(max_dist, xyz));
        self
    }
    
    pub fn further_than(mut self, min_dist: f64, pos: &impl IntoCoordinate) -> Self {
        if self.cached.is_some() { return self };
        let xyz = pos.as_coordinate();
        self.filter_set.push(QueryFilter::SpatialFurtherThan(min_dist, xyz));
        self
    }

    pub fn sort_filters(mut self) -> Self {
        if self.cached.is_some() { return self };
        self.filter_set.sort();
        self
    }

    pub fn make(mut self, local_world: &'a LocalWorld) -> Query<'a> {
        if let Some(cached) = self.cached {
            return cached.0
        }

        self.sort_and_prune_filters();

        Query {
            world: local_world,
            components: self.components,
            filter_set: self.filter_set,
        }
    }

    fn sort_and_prune_filters(&mut self) {
        self.filter_set.sort();
        for i in 0..self.filter_set.len() {
            match self.filter_set[i] {
                QueryFilter::ComponentAccess(_) => {
                    self.filter_set.truncate(i);
                    return
                },
                QueryFilter::ComponentWrite(_) => {
                    self.filter_set.truncate(i);
                    return
                },
                _ => {
                    continue;
                }
            }
        }
    }
}







#[derive(Debug)]
pub struct Query<'a> {
    world: &'a LocalWorld<'a>,
    components: Vec<ComponentSetId>,
    filter_set: Vec<QueryFilter>,
}

impl<'a> Query<'a> {
    pub fn new() -> QueryBuilder<'a> {
        QueryBuilder {
            filter_set: Vec::new(),
            components: Vec::new(),
            cached: None,
        }
    }
    
    pub fn cached(local_world: &'a LocalWorld) -> QueryBuilder<'a> {
        if local_world.cached_query_set().contains_key(&local_world.local_execution_id()) {
            // we've cached this query, fetch and return it
            unimplemented!()
        } else {
            // this is the first time we've seen this query
            QueryBuilder {
                filter_set: Vec::new(),
                components: Vec::new(),
                cached: None,
            }
        }
    }

    pub fn _execute(self) -> QueryResult<'a> {
        // - filter on spatial constraints and collect the list of entities
        // - test for the minimum constraint set, including the constrained spatial set
        // - begin iterating over the minimum set, test each constraint in minimum set order
        // - yield components of entities which satisfy all constraints

        let mut component_sets = Vec::new();
        for set_id in self.components {
            if let Some(set) = self.world.component_set_from_id(set_id) {
                component_sets.push(set)
            }
        }

        QueryResult {
            world: self.world,
            component_sets: component_sets,
        }
    }

    fn get_component_sets(&self) -> Vec<&'a ComponentSet> {
        let mut component_sets = Vec::new();
        for set_id in &self.components {
            if let Some(set) = self.world.component_set_from_id(*set_id) {
                component_sets.push(set)
            }
        }
        component_sets
    }
}





#[derive(Debug)]
pub struct QueryResult<'a> {
    world: &'a LocalWorld<'a>,
    component_sets: Vec<&'a ComponentSet>,
}

//macro_rules! impl_query_destructure {
//    ( $head:ident, $( $tail:ident, )* ) => {
//        impl<$head, $( $tail ),*> IntoQueryIter for ($head, $( $tail ),*)
//        {
//            // interesting delegation here, as needed
//        }
//
//        impl_query_destructure!($( $tail, )*);
//    };
//
//    () => {};
//}

//impl_query_destructure!(A,);

//impl<'a, A: Any> IntoQueryIter<'a, (A,)> for Query<'a> {
//    fn into_iter(self) -> QueryIter<'a, (A,)> {
//        let world = self.world;
//        let component_sets = self.get_component_sets();
//        let mut ordered_set = Vec::new();
//
//        for i in 0..component_sets.len() {
//            match component_sets[i].component_set_id() {
//                id if id == ComponentSetId::of::<A>() => {
//                    ordered_set.push(component_sets[i]);
//                },
//                _ => { continue; }
//            }       
//        }
//
//        return QueryIter::<(A,)> {
//            world: world,
//            required_components: ordered_set,
//            iteration_index: 0,
//            phantom: core::marker::PhantomData,
//        }
//    }
//}

// Lots of things to untangle:
//  - grab the sparsesets from the world in such a way that locks them when mutated
//  - robustly get and maintain knowledge of the hierarchy of minimum sets in the world
//  - implement query caching
//  - analyze the read/write pattern of each system and parallel run them where possible
//  - make adding component types to the world ridiculously simple, preferrably without having to "register" them at all

//impl<'a, A: Any, B: Any> IntoQueryIter<'a, (A, B)> for Query<'a> {
//    fn into_iter(self) -> QueryIter<'a, (A, B)> {
//        let world = self.world;
//        let component_sets = self.get_component_sets();
//        let mut ordered_set = Vec::new();
//
//        for i in 0..component_sets.len() {
//            match component_sets[i].component_set_id() {
//                id if id == ComponentSetId::of::<A>() => {
//                    ordered_set.push(component_sets[i]);
//                },
//                id if id == ComponentSetId::of::<B>() => {
//                    ordered_set.push(component_sets[i]);
//                },
//                _ => { continue; }
//            }
//        }
//
//        return QueryIter::<(A, B)> {
//            world: world,
//            ordered_components: ordered_set,
//            min_set_index: 0usize,
//            iteration_index: 0,
//            _phantom: core::marker::PhantomData,
//        }
//    }
//}


pub struct QueryIter<'a, T> {
    world: &'a LocalWorld<'a>,
    ordered_components: Vec<&'a ComponentSet>,
    min_set_index: usize,
    iteration_index: usize,
    maximum_iterations: usize,
    _phantom: core::marker::PhantomData<T>
}


#[allow(unused_macros)]
macro_rules! impl_into_query_iter {
    ($([$comp:ident, $index:expr]);*) => {
        #[allow(unused_parens)]
        #[allow(non_snake_case)]
        impl<'a, $($comp),*> IntoQueryIter<'a, ($($comp),*,)> for Query<'a>
        where $($comp: Debug + 'static),*
        {
            fn into_iter(&self) -> QueryIter<'a, ($($comp),*,)> {
                let mut iter = QueryIter {
                    world: self.world,
                    ordered_components: Vec::new(),
                    min_set_index: 0usize,
                    iteration_index: 0usize,
                    maximum_iterations: 0usize,
                    _phantom: ::core::marker::PhantomData, 
                };
                
                let required_components = self.get_component_sets();
                for set in required_components {
                    match set.component_set_id() {
                        $(
                            id if id == ComponentSetId::of::<$comp>() => {
                                iter.ordered_components.push(set);
                            },
                        )*
                        _ => { continue; }
                    }
                }

                if let ($(Some($comp)),*) = ($(iter.ordered_components.get($index).and_then(  |__set| __set.raw_set::<$comp>() )),*) {
                    iter.min_set_index = *[$($comp.len()),*].iter().min().expect("slice is not empty");
                    match iter.min_set_index {
                        $(
                            $index => {
                                iter.maximum_iterations = (iter.ordered_components.get($index).and_then(  |__set| __set.raw_set::<$comp>() )).expect("minimum set exists").len();
                            },
                        )*
                        _ => { unreachable!() }
                    }
                } else {
                    panic!("unable to populate QueryIter with all required components");
                }
                
                return iter;
            }
        }
    };
}

impl_into_query_iter!([A, 0]);
impl_into_query_iter!([A, 0]; [B, 1]);
impl_into_query_iter!([A, 0]; [B, 1]; [C, 2]);

#[allow(unused_macros)]
macro_rules! impl_query_iter {
    ($([$comp:ident, $index:expr]);*) => {
        #[allow(unused_parens)]
        #[allow(non_snake_case)]
        impl<'a, $($comp),*> Iterator for QueryIter<'a, ($($comp),*,)>
        where $($comp: Debug + 'static),*
        {
            type Item = ($(Ref<'a, $comp>),*);
            fn next(&mut self) -> Option<Self::Item> {
                while self.iteration_index < self.maximum_iterations {
                    // implement typeless raw index to entity ID over ComponentSet, thus we can convert the min set indices into IDs to access components in tuple order
                    return None
                }
                return None
            }
        }
    };
}

impl_query_iter!([A, 0]);
//impl_query_iter!([A, 0]; [B, 1]);
impl_query_iter!([A, 0]; [B, 1]; [C, 2]);


impl<'a, A, B> Iterator for QueryIter<'a, (A, B)>
where 
    A: Debug + 'static, 
    B: Debug + 'static,
{
    type Item = (Ref<'a, A>, Ref<'a, B>);

    fn next(&mut self) -> Option<Self::Item> {
        const IDX_A: usize = 0;
        const IDX_B: usize = 1;

        // first, get a reference to each of the component sets we're looking for
        let set_a = self.ordered_components.get(IDX_A).and_then(|set| set.raw_set::<A>());
        let set_b = self.ordered_components.get(IDX_B).and_then(|set| set.raw_set::<B>());

        // if we have all of the sets
        if let (Some(set_a), Some(set_b)) = (set_a, set_b) {

            // get the minimum set as an index of (A, B, C) etc. We need this to properly order the access
            //
            // better idea, 
            let min_set_idx = if set_a.len() > set_b.len() { IDX_B } else { IDX_A };

            match min_set_idx {
                IDX_A => {
                    
                    while self.iteration_index < set_a.len() {
                        let (entity, ca) = unsafe { set_a.get_kv(self.iteration_index).unwrap() };
                        
                        if let Some(cb) = set_b.get(entity) {
                            self.iteration_index += 1;
                            return Some((
                                Ref::new(ca, self.world),
                                Ref::new(cb, self.world),
                            ))
                        } else {
                            self.iteration_index += 1;
                            continue;
                        }
                    }      
                    return None;              
                },
                //IDX_B => {
                //    debug!("\t\tmatch IDX_B");
//
                //    let (entity, cb) = unsafe { set_b.get_kv(self.index).unwrap() };
                //    if let Some(ca) = set_a.get(entity) {
                //        Some((
                //            Ref::new(ca, self.world),
                //            Ref::new(cb, self.world),
                //        ))
                //    } else {
                //        None
                //    }
                //},
                _ => {
                    unreachable!();
                }
            };

            //self.index += 1;
            //return result;

        } else {
            None
        }
    }
}

pub trait IntoQueryIter<'a, T: Debug + 'static> {
    fn into_iter(&self) -> QueryIter<'a, T>;
}

/// A reference to a single component
/// 
/// This is a special mutable reference type. It is "lazy mutable".
///
/// All component queries are treated as immutable by default, however, as soon as
/// a `Ref` is mutably dereferenced, all access to the component type referenced by
/// the `Ref` for the encapsulating system are from then on are flagged as mutable
/// 
/// The first time this flag is raised, a re-evaluation of the systems dependency
/// graph is triggered. When the dependency graph is reconstructed, it's possible
/// that some running systems data may be invalidated and must be calculated again
#[derive(Debug)]
pub struct Ref<'a, T> {
    target: UnsafeCell<&'a T>,
    world: &'a LocalWorld<'a>,
}

impl<'a, T> Ref<'a, T> {
    fn new(item: &'a T, world: &'a LocalWorld) -> Self {
        Ref {
            target: UnsafeCell::new(item),
            world: world,
        }
    }
}

impl<'a, T> Deref for Ref<'a, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        unsafe { *self.target.get() }
    }
}

impl<'a, T> DerefMut for Ref<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unimplemented!()

        // If this is the first time mutably dereferencing from this system, flag the system
        // as mutably borrowing the specified component and rebuild the dependency graph.
        // Otherwise, we've already flagged this and we can just return the mutable reference
        // 
        // Note: Explore swapping a function pointer between code paths vs performing a comparison
        //       for every deref when deciding whether this is the first mutable access. Explore
        //       elevating if/else into higher level structural changes. Compare performance
        //       against branch with `core::intrinsics::likely` compiler hint
    }
}

#[cfg(test)]
mod test_query {
    use super::*;

    #[test]
    fn test_query_iter() {
        let dummy_world = crate::world::World::test_dummy();
        let local_world = LocalWorld::test_dummy(&dummy_world);
        let query = Query::new()
            .with::<i32>()
            .with::<u64>()
            .not::<bool>()
            .changed::<i32>()
            .make(&local_world);

        //println!("{:#?}", query);
    }
}


// Notes
//
//  - Queries have two flavors (for now), EntityWise and ComponentWise
//    > EntityWise collect a set of EntityId's and then filter on components
//    > ComponentWise collect a set of component types and then filter on component matches
