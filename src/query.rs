use std::{any::{Any, TypeId}, fmt::Debug, ops::Deref};

use crate::{
    identity::{EntityId, LocalExecutionId}, 
    world::{ComponentSet, ComponentSetId, LocalWorld, Spatial},
    collections::{Get, GetMut},
};





#[derive(Debug, Copy, Clone)]
pub enum QueryFilter {
    ComponentAnyChanged(ComponentSetId), // rejects all entities if none of their given component has changed
    ComponentChanged(ComponentSetId), // passes only the entities which have had the given component change since the last query in this system
    ComponentRead(ComponentSetId), // passes all entities with the given component, with read access to that component
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
            QueryFilter::ComponentRead(_) => 1000,
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
    read_set: Vec<ComponentSetId>,
    write_set: Vec<ComponentSetId>,
    cached: Option<(Query<'a>, &'a LocalWorld<'a>)>,
}

impl<'a> QueryBuilder<'a> {
    pub fn read<T>(mut self) -> Self where T: 'static {
        if self.cached.is_some() { return self };

        let id = ComponentSetId::of::<T>();
        self.filter_set.push(QueryFilter::ComponentRead(id));
        self.read_set.push(id);
        self
    }

    pub fn write<T>(mut self) -> Self where T: 'static {
        if self.cached.is_some() { return self };

        let id = ComponentSetId::of::<T>();
        self.filter_set.push(QueryFilter::ComponentWrite(id));
        self.write_set.push(id);
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

    pub fn closer_than(mut self, max_dist: f64, pos: &impl Spatial) -> Self {
        if self.cached.is_some() { return self };
        let xyz = pos.position();
        self.filter_set.push(QueryFilter::SpatialCloserThan(max_dist, xyz));
        self
    }
    
    pub fn further_than(mut self, min_dist: f64, pos: &impl Spatial) -> Self {
        if self.cached.is_some() { return self };
        let xyz = pos.position();
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
            local_world: local_world,
            read_set: self.read_set,
            write_set: self.write_set,
            filter_set: self.filter_set,
        }
    }

    fn sort_and_prune_filters(&mut self) {
        self.filter_set.sort();
        for i in 0..self.filter_set.len() {
            match self.filter_set[i] {
                QueryFilter::ComponentRead(_) => {
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
    local_world: &'a LocalWorld<'a>,
    read_set: Vec<ComponentSetId>,
    write_set: Vec<ComponentSetId>,
    filter_set: Vec<QueryFilter>,
}

impl<'a> Query<'a> {
    pub fn new() -> QueryBuilder<'a> {
        QueryBuilder {
            filter_set: Vec::new(),
            read_set: Vec::new(),
            write_set: Vec::new(),
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
                read_set: Vec::new(),
                write_set: Vec::new(),
                cached: None,
            }
        }
    }

    pub fn execute(self) -> QueryResult<'a> {
        // - filter on spatial constraints and collect the list of entities
        // - test for the minimum constraint set, including the constrained spatial set
        // - begin iterating over the minimum set, test each constraint in minimum set order
        // - yield components of entities which satisfy all constraints

        let mut component_sets = Vec::new();
        for set_id in self.read_set {
            if let Some(set) = self.local_world.component_set_from_id(set_id) {
                component_sets.push(set)
            }
        }

        QueryResult {
            component_sets: component_sets,
        }
    }
}





#[derive(Debug)]
pub struct QueryResult<'a> {
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

impl<'a, A: Any> IntoQueryIter<'a, (A,)> for QueryResult<'a> {
    fn iter(&self) -> QueryIter<'a, (A,)> {
        let mut ordered_set = Vec::new();
        for i in 0..self.component_sets.len() {
            match self.component_sets[i].component_set_id() {
                id if id == ComponentSetId::of::<A>() => {
                    ordered_set.push(self.component_sets[i]);
                },
                _ => { continue; }
            }       
        }

        return QueryIter::<(A,)> {
            ordered_set: ordered_set,
            index: 0,
            phantom: core::marker::PhantomData,
        }
    }
}

// Lots of things to untangle:
//  - grab the sparsesets from the world in such a way that locks them when mutated
//  - robustly get and maintain knowledge of the hierarchy of minimum sets in the world
//  - implement query caching
//  - analyze the read/write pattern of each system and parallel run them where possible
//  - make adding component types to the world ridiculously simple, preferrably without having to "register" them at all

impl<'a, A: Any, B: Any> IntoQueryIter<'a, (A, B)> for QueryResult<'a> {
    fn iter(&self) -> QueryIter<'a, (A, B)> {
        let mut ordered_set = Vec::new();
        for i in 0..self.component_sets.len() {
            match self.component_sets[i].component_set_id() {
                id if id == ComponentSetId::of::<A>() => {
                    ordered_set.push(self.component_sets[i]);
                },
                id if id == ComponentSetId::of::<B>() => {
                    ordered_set.push(self.component_sets[i]);
                },
                _ => { continue; }
            }
        }

        return QueryIter::<(A, B)> {
            ordered_set: ordered_set,
            index: 0,
            phantom: core::marker::PhantomData,
        }
    }
}

//impl<'a, A: core::any::Any, B: core::any::Any, C: core::any::Any> IntoQueryIter<'a, (A, B, C)> for QueryResult<'a> {
//    fn iter(&self) -> QueryIter<'a, (A, B, C)> {
//        unimplemented!()
//    }
//}





pub struct QueryIter<'a, T> {
    ordered_set: Vec<&'a ComponentSet>,
    index: usize,
    phantom: core::marker::PhantomData<T>
}

impl<'a, A> Iterator for QueryIter<'a, (A,)>
where
    A: Debug + 'static
{
    type Item = (Ref<'a, A>, );

    fn next(&mut self) -> Option<Self::Item> {
        self.index += 1;
        if let Some(minimum_set) = self.ordered_set.first() {
            if let Some(raw_set) = minimum_set.raw_set::<A>() {
                if let Some(component) = raw_set.as_slice().get(self.index - 1) {
                    return Some((Ref { target: component}, ));
                } else {
                    None
                }                
            } else {
                None
            }
        } else {
            None
        }
    }
}

impl<'a, A, B> Iterator for QueryIter<'a, (A, B)>
where 
    A: Debug + 'static, 
    B: Debug + 'static,
{
    type Item = (Ref<'a, A>, Ref<'a, B>);

    fn next(&mut self) -> Option<Self::Item> {
        unimplemented!()
    }
}

pub trait IntoQueryIter<'a, T: Any> {
    fn iter(&self) -> QueryIter<'a, T>;
}

#[derive(Debug)]
pub struct Ref<'a, T> {
    target: &'a T,
}

#[cfg(test)]
mod test_query {
    use super::*;

    #[test]
    fn test_query_iter() {
        let dummy_world = crate::world::World::test_dummy();
        let local_world = LocalWorld::test_dummy(&dummy_world);
        let query = Query::new()
            .read::<i32>()
            .write::<u64>()
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
