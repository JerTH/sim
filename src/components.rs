use std::{any::Any, cell::UnsafeCell, fmt::{ Debug, Display }};
use unsafe_any::UnsafeAnyExt;

use crate::{collections::{Get, GetMut, SparseSet}, identity::{InternalTypeId, LinearId}, world::EntityId};

type Generation = u32;
type RawComponentSet<T> = SparseSet<UnsafeCell<T>>;

pub trait Component: Debug + Any + 'static {} 
impl<T> Component for T where T: Debug + Any + 'static {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ComponentSetId(InternalTypeId);

impl Display for ComponentSetId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let v: usize = self.0.into();
        write!(f, "{}", v)
    }
}

impl Into<usize> for ComponentSetId {
    fn into(self) -> usize {
        self.0.as_linear_u64() as usize
    }
}

impl ComponentSetId {
    pub(crate) fn of<T>() -> Self where T: 'static {
        ComponentSetId(InternalTypeId::of::<SparseSet<T>>())
    }
}

//impl Get<ComponentSetId> for SparseSet<ComponentSet> {
//    type Item = ComponentSet;
//    fn get(&self, idx: ComponentSetId) -> Option<&Self::Item> {
//        self.get(idx.as_linear_u64() as usize)
//    }
//}
//
//impl GetMut<ComponentSetId> for SparseSet<ComponentSet> {
//    type Item = ComponentSet;
//    fn get_mut(&mut self, idx: ComponentSetId) -> Option<&mut Self::Item> {
//        self.get_mut(idx.as_linear_u64() as usize)
//    }
//}

// TODO: - Can we make ComponentSet's safe by integrating some conflict-tracking mechanism into them?
//         In the same way that a RefCell tracks borrows, a component set could track how it's being accessed
//         by systems. A system might request access to the ComponentSet using its "clique number"
//         so long as everything requesting access uses the same clique number, access is granted.
//         Idea worth exploring further, maybe better solutions
#[derive(Debug)]
pub struct ComponentSet {
    id: ComponentSetId, // relates the set back to the typeid of SparseSet<Component>
    count: usize,
    set: Box<dyn Any>, // SparseSet<Component, EntityId>
    
    // for debug display purposes
    name: &'static str,
}

impl Display for ComponentSet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let id: usize = self.id.into();
        write!(f, "ComponentSet(\"{name}\", count={count}, id={id})", id=id, name=self.name, count=self.count)
    }
}

// Send and Sync because we guarantee that data races don't happen in the world via the conflict graph and system tree
unsafe impl Send for ComponentSet {}
unsafe impl Sync for ComponentSet {}

impl ComponentSet {
    fn new<T: Debug>() -> Self where T: 'static {
        ComponentSet {
            count: 0,
            id: ComponentSetId::of::<T>(),
            name: std::any::type_name::<T>(),
            set: Box::new(RawComponentSet::<T>::new()),
        }
    }

    fn len<T: Component>(&self) -> usize {
        if let Some(sparse) = self.set.downcast_ref::<RawComponentSet<T>>() {
            sparse.len()
        } else {
            0usize
        }
    }
    
    #[deprecated] // this sounds too similar to adding a component to the world. we're at a lower level here
    fn insert<T: Component>(&mut self, index: usize, component: T) -> Result<(), ()> {
        match self.set.downcast_mut::<RawComponentSet<T>>() {
            Some(sparse) => {
                sparse.insert_with(index, UnsafeCell::new(component));
            },
            None => {

            }
        }

        //if self.set.is::<RawComponentSet<T>>() {
        //    if let Some(set) = self.set.downcast_mut::<RawComponentSet<T>>() {
        //        let result = set.insert_with(entity.into(), UnsafeCell::new(component));
        //        
        //        assert!(result.is_none());
        //        self.count += 1;
        //    } else {
        //        panic!("ComponentSet::set downcast failed");
        //    }
        //} else {
        //    panic!("ComponentSet::set is not SparseSet<T>");
        //}
        
        todo!() // revise this
    }
    
    pub fn contains<T>(&self) -> bool where T: 'static {
        self.id == ComponentSetId::of::<T>()
    }

    pub(crate) fn id(&self) -> ComponentSetId {
        self.id
    }
    
    pub(crate) fn raw_set<T: 'static>(&self) -> Option<&RawComponentSet<T>> {
        self.set.downcast_ref::<RawComponentSet<T>>()
    }

    pub(crate) unsafe fn raw_set_unchecked<T: 'static>(&self) -> &RawComponentSet<T> {
        self.set.downcast_ref_unchecked::<RawComponentSet<T>>()
    }
}
