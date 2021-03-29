/// Typeid

use core::panic;
use std::{cmp::Ordering, hash::Hash};
use std::sync::{ RwLock, Once };
use std::sync::atomic::{ AtomicU64, AtomicUsize };
use std::collections::HashMap;
use std::cell::Cell;

type TypeIdMap = HashMap<core::any::TypeId, InternalTypeId>;
struct TypeIdMapRwLock(Cell<Option<RwLock<TypeIdMap>>>);

static TYPEID_INIT_ONCE: Once = Once::new();

static mut TYPEID_MAP: TypeIdMapRwLock = TypeIdMapRwLock::new();
static mut TYPEID_COUNTER: AtomicUsize = AtomicUsize::new(0);
static mut INSTANCEID_COUNTER: AtomicU64 = AtomicU64::new(0);
static mut ENTITYID_COUNTER: AtomicU64 = AtomicU64::new(0);

/// An opaque identifier for any given entity in the world. Corresponds to exactly one entity, alive or dead.
struct EntityId(u32);

impl EntityId {
    pub fn new() -> EntityId {
        unimplemented!()
    }
}

/// Uniquely identifying opaque ID which can be used to differentiate instances of otherwise identical structures,
/// or uniquely group a set of instances with an identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InstanceId(u64);

impl InstanceId {
    pub fn unique() -> Self {
        InstanceId (unsafe { INSTANCEID_COUNTER.fetch_add(1u64, core::sync::atomic::Ordering::SeqCst) } )
    }
}

impl TypeIdMapRwLock {
    const fn new() -> Self {
        TypeIdMapRwLock(Cell::new(None))
    }
}

fn init_typeid_map() -> RwLock<TypeIdMap> {
    RwLock::new(core::default::Default::default())
}

unsafe fn get_typeid_map() -> &'static RwLock<TypeIdMap> {
    TYPEID_INIT_ONCE.call_once(|| {
        TYPEID_MAP.0.set(Some(init_typeid_map()))
    });
    
    match *TYPEID_MAP.0.as_ptr() {
        Some(ref typeid_map) => {
            typeid_map
        },
        None => {
            panic!("Attempt to reference uninitialized global typeid map");
        }
    }
}

impl std::ops::Deref for TypeIdMapRwLock {
    type Target = RwLock<TypeIdMap>;
    fn deref(&self) -> &'static RwLock<TypeIdMap> {
        unsafe { get_typeid_map() }
    }
}

/// Lazily allocated and lazily assigned, types are assigned a unique ID the first time it's requested
/// ID's are as unique as `core::any::TypeId`, but are linear and small in integer value proportional to the number of ID's requested.
/// This is useful in certain circumstances where having type ID's which can be represented in a small number of bytes (one or two) is desired
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct InternalTypeId(usize);

impl PartialOrd for InternalTypeId {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        None // no-op, must be implemented to satisfy ordering elsewhere, but type-ids are considered unordered
    }
}

impl Ord for InternalTypeId {
    fn cmp(&self, other: &Self) -> Ordering {
        Ordering::Equal // all type-id's are considered equal in ordering, they are unordered
    }
}

impl InternalTypeId {
    pub fn of<T>() -> Self where T: 'static {
        type_id::<T>()
    }

    pub fn total_assigned() -> usize {
        unsafe {
            let len = TYPEID_MAP.read().expect("Attempt to reference uninitialized typeid map").len();
            loop {
                let count = TYPEID_COUNTER.load(core::sync::atomic::Ordering::SeqCst);
                
                if count == len {
                    return count;
                }
            }
        }
    }

    pub fn raw(&self) -> usize {
        self.0
    }
}

fn type_id<T>() -> InternalTypeId where T: 'static {
    let tid = core::any::TypeId::of::<T>();
    let iid;
    unsafe {
        if let Ok(mut guard) = TYPEID_MAP.write() {
            if let Some(type_id) = guard.get(&tid) {
                return *type_id
            } else {
                iid = InternalTypeId(TYPEID_COUNTER.fetch_add(1usize, core::sync::atomic::Ordering::SeqCst));
                if let Some(v) = guard.insert(tid, iid) {
                    panic!("TypeId already mapped {:?}", v);
                }
                
                assert!(!guard.is_empty());
                assert_eq!(guard.len() - 1, iid.raw());

            }
        } else {
            panic!("Attempt to reference uninitialized global typeid map");
        }
    }
    return iid
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_internal_type_ids() {
        struct A;
        struct B(usize);
        struct C<T>(T);

        assert_eq!(0, InternalTypeId::total_assigned());
        
        assert_eq!(0, InternalTypeId::of::<A>().raw());
        assert_eq!(1, InternalTypeId::of::<B>().raw());
        assert_eq!(0, InternalTypeId::of::<A>().raw());
        assert_eq!(1, InternalTypeId::of::<B>().raw());
        assert_eq!(2, InternalTypeId::of::<u32>().raw());
        assert_eq!(3, InternalTypeId::of::<f32>().raw());
        assert_eq!(4, InternalTypeId::of::<C<A>>().raw());
        assert_eq!(5, InternalTypeId::of::<C<B>>().raw());
        assert_eq!(6, InternalTypeId::of::<C<C<A>>>().raw());
        assert_eq!(7, InternalTypeId::of::<C<C<(A,B)>>>().raw());
        
        assert_eq!(8, InternalTypeId::total_assigned());
    }
}
