use std::{any::Any, cell::UnsafeCell, fmt::{ Debug, Display }};
use unsafe_any::UnsafeAnyExt;

use crate::{collections::{Get, GetMut, SparseSet}, identity::{EntityId, InternalTypeId, LinearId}};



pub trait Component: Debug + Any + 'static {} 
impl<T> Component for T where T: Debug + Any + 'static {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

impl LinearId for ComponentSetId {
    fn unique() -> Self {
        ComponentSetId(InternalTypeId::unique())
    }

    fn as_linear_u64(&self) -> u64 {
        self.0.as_linear_u64()
    }
}


impl Get<ComponentSetId> for SparseSet<ComponentSet> {
    type Item = ComponentSet;
    fn get(&self, idx: ComponentSetId) -> Option<&Self::Item> {
        self.get(idx.as_linear_u64() as usize)
    }
}

impl GetMut<ComponentSetId> for SparseSet<ComponentSet> {
    type Item = ComponentSet;
    fn get_mut(&mut self, idx: ComponentSetId) -> Option<&mut Self::Item> {
        self.get_mut(idx.as_linear_u64() as usize)
    }
}

type RawComponentSet<T> = SparseSet<UnsafeCell<T>>;

#[derive(Debug)]
pub struct ComponentSet {
    ident: ComponentSetId, // relates the set back to the typeid of SparseSet<Component>
    count: usize,
    name: String,
    set: Box<dyn Any>, // SparseSet<Component, EntityId>
}

// Send and Sync because we guarantee that data races don't happen in the world via the conflict graph and system tree
unsafe impl Send for ComponentSet {}
unsafe impl Sync for ComponentSet {}

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
