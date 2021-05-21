/// Query

// TODO: - Executing queries currently uses a fair amount of unsafe code, this could be a good target for
//         improvement in the future
//       - More complex queries, spatial constraints, time constraints
//       - Query caching

use std::{fmt::Debug, ops::{Deref, DerefMut}};

use crate::{collections::{Get}, components::{Component, ComponentSet, ComponentSetId}, debug::*, world::{IntoCoordinate, LocalWorld}};
use crate::systems::DependencyType;

#[derive(Debug, Copy, Clone)]
pub enum QueryFilter {
    ComponentAnyChanged(ComponentSetId), // rejects all entities if none of their given component has changed
    ComponentChanged(ComponentSetId), // passes only the entities which have had the given component change since the last query in this system
    ComponentAccess(ComponentSetId), // passes all entities with the given component, with read access to that component
    ComponentWrite(ComponentSetId), // passes all entities with the given component, with write access to that component
    ComponentNot(ComponentSetId), // rejects all entities with the given component
    SpatialCloserThan(f64, (f64, f64, f64)), // rejects all entities which can be considered further than the given linear distance from the given 3D point
    SpatialFurtherThan(f64, (f64, f64, f64)), // rejects all entities which can be considered closer than the given linear distance from the given 3D point
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
    pub fn with<T>(mut self) -> Self where T: Component {
        if self.cached.is_some() { return self };

        let id = ComponentSetId::of::<T>();
        self.filter_set.push(QueryFilter::ComponentAccess(id));
        self.components.push(id);
        self
    }
    
    pub fn not<T>(mut self) -> Self where T: Component {
        if self.cached.is_some() { return self };
        self.filter_set.push(QueryFilter::ComponentNot(ComponentSetId::of::<T>()));
        self
    }

    pub fn changed<T>(mut self) -> Self where T: Component {
        if self.cached.is_some() { return self };
        self.filter_set.push(QueryFilter::ComponentChanged(ComponentSetId::of::<T>()));
        self
    }

    pub fn any_changed<T>(mut self) -> Self where T: Component {
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
        if local_world.cached_query_set().contains_key(&local_world.system_id()) {
            // we've cached this query, fetch and return it
            todo!()
        } else {
            // this is the first time we've seen this query
            QueryBuilder {
                filter_set: Vec::new(),
                components: Vec::new(),
                cached: None,
            }
        }
    }
    
    /// Collects the required components from the world and returns them as raw pointers
    ///
    /// SAFETY:
    ///
    /// Because components are only ever accessed in parallel by systems, safety is
    /// guaranteed though the system tree, which only allows systems to run in parallel
    /// which satisfy mutability rules
    unsafe fn get_required_components<'b>(&self) -> Vec<*mut ComponentSet> {
        let mut components = Vec::new();

        for component_set_id in self.components.iter() {
            match self.world.get_component_set(*component_set_id) {
                Some(guard) => {
                    components.push(guard);
                },
                None => {
                    warn!("failed to lock required component set");
                }
            }
        }
        components
    }
}

pub struct QueryIter<'a, T> {
    world: &'a LocalWorld<'a>,
    ordered_components: Vec<*mut ComponentSet>,
    min_set_index: usize,
    iteration_index: usize,
    maximum_iterations: usize,
    _phantom: core::marker::PhantomData<T>
}

pub trait IntoQueryIter<'a, T> {
    unsafe fn into_iter(&self) -> QueryIter<'a, T>;
}

/// Implements the transformation from a Query into a QueryIter
#[allow(unused_macros)]
macro_rules! impl_into_query_iter {
    ($([$comp:ident, $index:expr]);*) => {

        #[allow(unused_parens)]
        #[allow(non_snake_case)]
        impl<'a, $($comp),*> IntoQueryIter<'a, ($($comp),*,)> for Query<'a>
        where $($comp: Component),*
        {
            // revised 18/05/2021
            /// Turns a Query into a QueryIter
            /// 
            /// SAFETY:
            ///
            /// THIS IS NOT SAFE TO USE. EVER.
            /// 
            /// SAFETY IS ONLY GUARANTEED WHEN MANAGED PROPERLY BY A `WORLD` INSTANCE
            ///
            /// DO NOT USE THIS FUNCTION OTHERWISE
            unsafe fn into_iter(&self) -> QueryIter<'a, ($($comp),*,)> {
                let mut __iter = QueryIter {
                    world: self.world,
                    ordered_components: Vec::new(),
                    min_set_index: 0usize,
                    iteration_index: 0usize,
                    maximum_iterations: 0usize,
                    _phantom: ::core::marker::PhantomData, 
                };
                
                let __required_components = self.get_required_components();
                $(
                    for __required_component in __required_components.iter() { // TODO: optimize this macro/loop to reduce query overhead
                        let __macro_invocation_component_id = ComponentSetId::of::<$comp>();
                        
                        // UNSAFE POINTER DEREFERENCE. WILL BREAK THINGS IF THIS FUNCTION IS USED IMPORPERLY
                        let __required_component_id = (**__required_component).id();

                        if __required_component_id == __macro_invocation_component_id {

                            // push components in the order they are expected
                            __iter.ordered_components.push(*__required_component);

                            // any component access flags the system as having a read dependency on that component
                            self.world.mark_dependency(DependencyType::Read, __macro_invocation_component_id);
                        }
                    }
                )*

                // expands into a tuple which captures all of the required components in their expected places
                let __ordered_components = (
                    $(
                        __iter.ordered_components.get($index).and_then(|__set| {
                            
                            // UNSAFE POINTER DEREFERENCE. WILL BREAK THINGS IF THIS FUNCTION IS USED IMPORPERLY
                            let s = unsafe { &**__set };
                            
                            s.raw_set::<$comp>()
                        })
                    ),*
                );

                // the if let here expands into a tuple of Some()'s, this is shorthand for "do we have all of the components we asked for?"
                if let ($(Some($comp)),*) = __ordered_components {
                    // figure out which component in our ordered set has the fewest elements, it's the anchor for iteration

                    // this expands into an iterable array of the number of components in each set, we then get the index of the set
                    // with the fewest number of components in it, __iter.min_set_index is then assigned to this index
                    __iter.min_set_index = match [$($comp.len()),*].iter().enumerate().min_by_key(|(_idx, &len)| {
                        len // min by key is interested in the number of components in the set
                    }) {
                        Some(item) => {
                            item.0 // our item created by enumerate() is a (idx, len) for each component, we want the index
                        },
                        None => {
                            todo!("better error handling here");
                        }
                    };

                    match __iter.min_set_index {
                        $(
                            $index => {
                                // match on the component set with the fewest elements and set the iterators max iterations to its length 
                                __iter.maximum_iterations = match (__iter.ordered_components.get($index).and_then(|__set| {
                                    
                                    // UNSAFE POINTER DEREFERENCE. WILL BREAK THINGS IF THIS FUNCTION IS USED IMPORPERLY
                                    let s = unsafe { &**__set };
                                    
                                    s.raw_set::<$comp>()
                                })) {
                                    Some(__min_component_set) => {
                                        // the number of components in the component set with the fewest elements
                                        __min_component_set.len()
                                    },
                                    None => {
                                        todo!("better error handling here");
                                    }
                                };
                            },
                        )*
                        _ => { unreachable!() }
                    }
                } else {
                    todo!("don't make this a fatal error, log the error and return an empty iterator. don't want engine crashing all of the time");
                    //fatal!("Unable to populate QueryIter with all required components: {:?}", __ordered_components);
                }

                // return the finished iterator
                return __iter;
            }
        }
    };
}

impl_into_query_iter!([A, 0]);
impl_into_query_iter!([A, 0]; [B, 1]);
impl_into_query_iter!([A, 0]; [B, 1]; [C, 2]);
impl_into_query_iter!([A, 0]; [B, 1]; [C, 2]; [D, 3]);
impl_into_query_iter!([A, 0]; [B, 1]; [C, 2]; [D, 3]; [E, 4]);
impl_into_query_iter!([A, 0]; [B, 1]; [C, 2]; [D, 3]; [E, 4]; [F, 5]);

#[allow(unused_macros)]
macro_rules! impl_query_iter {
    ($([$comp:ident, $index:expr]);*) => {
        #[allow(unused_parens)]
        #[allow(non_snake_case)]
        #[allow(unreachable_patterns)]
        impl<'a, $($comp),*> Iterator for QueryIter<'a, ($($comp),*,)>
        where $($comp: Component),*
        {
            type Item = ($(Ref<'a, $comp>),*);
            fn next(&mut self) -> Option<Self::Item> {
                while self.iteration_index < self.maximum_iterations {
                    let entity_id: usize = match self.min_set_index {
                        $(
                            $index => {
                                unsafe {
                                    // UNSAFE POINTER DEREFERENCE
                                    let __component_set = &(*self.ordered_components[$index]);
                                    
                                    // raw_set_unchecked is unsafe
                                    if let Some(entity_id) = __component_set.raw_set_unchecked::<$comp>().get_key(self.iteration_index) {
                                        self.iteration_index += 1;
                                        entity_id
                                    } else {
                                        return None
                                    }
                                }
                            },
                        )*
                        _ => unreachable!()
                    };

                    // expands into named 
                    $(
                        let $comp: Ref<$comp> = {
                            let __component = unsafe { (*self.ordered_components[$index]).raw_set_unchecked::<$comp>().get(entity_id) };

                            if let Some(__component) = __component {
                                Ref::new(__component.get(), Some(self.world)) // TODO: Need some better mechanism for quickly setting a write dep and checking it
                            } else {
                                return None; // early return
                            }
                        };
                    )*

                    return Some(($($comp),*));
                }
                return None
            }
        }
    };
}

impl_query_iter!([A, 0]);
impl_query_iter!([A, 0]; [B, 1]);
impl_query_iter!([A, 0]; [B, 1]; [C, 2]);
impl_query_iter!([A, 0]; [B, 1]; [C, 2]; [D, 3]);
impl_query_iter!([A, 0]; [B, 1]; [C, 2]; [D, 3]; [E, 4]);
impl_query_iter!([A, 0]; [B, 1]; [C, 2]; [D, 3]; [E, 4]; [F, 5]);

/// A reference to a single component
/// 
/// This is a special mutable reference type. It is "lazy mutable".
///
/// All component queries are treated as immutable by default, however, as soon as
/// a `Ref` is mutably dereferenced, all access to the component type referenced by
/// the `Ref` for the encapsulating system are from then on flagged as mutable
/// 
/// The first time this flag is raised, a re-evaluation of the systems dependency
/// graph is triggered. When the dependency graph is reconstructed, it's possible
/// that some running systems data may be invalidated and must be calculated again
#[derive(Debug)]
pub struct Ref<'a, T> {
    target: *mut T,

    // Some if the reference is used as immutable, None if the system is known to mutate this component
    // 
    // TODO: Investigate moving this to a thread-local or static check on writes, or some other mechanism which speeds up reference iteration
    //       CONSIDER: limiting each system to a single query?
    world: Option<&'a LocalWorld<'a>>,
}

impl<'a, T> Ref<'a, T> {
    fn new(item: *mut T, world: Option<&'a LocalWorld>) -> Self {
        Ref {
            target: item,
            world: world,
        }
    }
}

impl<'a, T> Deref for Ref<'a, T> where T: Component {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        // Read dependencies are implicit based on the inclusion of a component in a query, thus we don't need to check here
        unsafe {
            // Safety: Our pointer is into SparseSet<UnsafeCell<T>>::data, these are owned values and the data is guaranteed to be non-null
            &*(self.target)
        }
    }
}

impl<'a, T> DerefMut for Ref<'a, T> where T: Component  {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target {
        // Write dependencies are more complicated, they are lazy and must be marked the very first time a given system
        // uses a component mutably. This only happens once for the life of the system.
        if let Some(world) = self.world {
            //debug!("marking write dependency for {:?} in system {:?}", std::any::type_name::<T>(), world.system_execution_id());
            world.mark_dependency(DependencyType::Write, ComponentSetId::of::<T>());
        }
        
        unsafe {
            &mut (*self.target)
        }

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
mod tests {
}


// Notes
//
//  - Queries have two flavors, entity-wise and component-wise
//    - Entity-wise collect a set of EntityId's and then optionally filter on components
//    - Component-wise collect a set of component types and then filter on component matches
// 
//  - Pseudo Components. Components which are built in, and may not actually store data, but
//    expose some special functionality to systems
//    
