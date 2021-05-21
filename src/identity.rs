/// Identity

// TODO: - Provide some mechanism to associate InternalTypeId's with a string name for debug and logging purposes

use core::panic;
use std::{cell::Cell, cmp::Ordering, collections::HashMap, fmt::Display, hash::Hash, ops::Deref, sync::{atomic::{self, AtomicU64}, RwLock, Once}};

#[cfg_attr(any(target_arch="x86_64", target_arch="aarch64"), repr(align(128)))]
#[cfg_attr(not(any(target_arch="x86_64", target_arch="aarch64")), repr(align(64)))]
struct PaddedAtomicU64(AtomicU64);

impl PaddedAtomicU64 {
    const fn new(val: u64) -> Self {
        PaddedAtomicU64(AtomicU64::new(val))
    }
}

impl Deref for PaddedAtomicU64 {
    type Target = AtomicU64;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

static TYPEID_INIT_ONCE: Once = Once::new();

static mut TYPEID_MAP: TypeIdMapRwLock = TypeIdMapRwLock::new();
static mut TYPEID_COUNTER: PaddedAtomicU64 = PaddedAtomicU64::new(0);
static mut INSTANCEID_COUNTER: PaddedAtomicU64 = PaddedAtomicU64::new(0);
static mut LOCALEXECUTIONID_COUNTER: PaddedAtomicU64 = PaddedAtomicU64::new(0);

pub(crate) trait LinearId {
    fn unique() -> Self;
    fn as_linear_u64(&self) -> u64;
}

/// An opaque identifier used to keep track of the context of execution. This is used to uniquely identify systems,
/// and conduct appropriate caching and filtering for query's. Modifying the contents is considered an error under all circumstances
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct SystemExecutionId(usize);

impl LinearId for SystemExecutionId {
    fn unique() -> Self {
        SystemExecutionId(unsafe { LOCALEXECUTIONID_COUNTER.fetch_add(1, atomic::Ordering::SeqCst) as usize })
    }

    fn as_linear_u64(&self) -> u64 {
        self.0 as u64
    }
}


/// Uniquely identifying opaque ID which can be used to differentiate instances of otherwise identical structures,
/// or uniquely group a set of instances with an identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InstanceId(u64);

impl LinearId for InstanceId {
    fn unique() -> Self {
        InstanceId (unsafe { INSTANCEID_COUNTER.fetch_add(1u64, core::sync::atomic::Ordering::SeqCst) } )
    }

    fn as_linear_u64(&self) -> u64 {
        self.0
    }
}

struct TypeIdMapRwLock(Cell<Option<RwLock<HashMap<core::any::TypeId, InternalTypeId>>>>);

impl TypeIdMapRwLock {
    const fn new() -> Self {
        TypeIdMapRwLock(Cell::new(None))
    }
}

fn init_typeid_map() -> RwLock<HashMap<core::any::TypeId, InternalTypeId>> {
    RwLock::new(core::default::Default::default())
}

unsafe fn get_typeid_map() -> &'static RwLock<HashMap<core::any::TypeId, InternalTypeId>> {
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
    type Target = RwLock<HashMap<core::any::TypeId, InternalTypeId>>;
    fn deref(&self) -> &'static Self::Target {
        unsafe { get_typeid_map() }
    }
}

/// Lazily allocated and lazily assigned, types are assigned a unique ID the first time it's requested
/// ID's are as unique as `core::any::TypeId`, but are linear and small in integer value proportional to the number of ID's requested.
/// This is useful in certain circumstances where having type ID's which can be represented in a small number of bytes (one or two) is desired
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct InternalTypeId(u64);

impl InternalTypeId {
    pub fn of<T>() -> Self where T: 'static {
        type_id::<T>()
    }

    #[allow(dead_code)]
    #[deprecated]
    fn _total_assigned() -> usize {
        unsafe {
            let len = TYPEID_MAP.read().expect("Attempt to reference uninitialized typeid map").len();
            loop {
                let count = TYPEID_COUNTER.load(core::sync::atomic::Ordering::SeqCst);
                
                if count == len as u64 {
                    return count as usize;
                }
            }
        }
    }
}

impl Into<usize> for InternalTypeId {
    fn into(self) -> usize {
        self.0 as usize
    }
}

impl PartialOrd for InternalTypeId {
    fn partial_cmp(&self, _: &Self) -> Option<Ordering> {
        None // no-op, must be implemented to satisfy ordering elsewhere, e.g. in query sorting, but type-ids are considered unordered
    }
}

impl Ord for InternalTypeId {
    fn cmp(&self, _: &Self) -> Ordering {
        Ordering::Equal // all type-id's are considered equal in ordering, they are unordered
    }
}

impl LinearId for InternalTypeId {
    fn unique() -> InternalTypeId {
        InternalTypeId::of::<()>()
    }

    fn as_linear_u64(&self) -> u64 {
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
                iid = InternalTypeId(TYPEID_COUNTER.fetch_add(1, core::sync::atomic::Ordering::SeqCst));
                if let Some(v) = guard.insert(tid, iid) {
                    panic!("TypeId already mapped {:?}", v);
                }
                
                assert!(!guard.is_empty());
                assert_eq!(guard.len() - 1, iid.as_linear_u64() as usize);

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

        let tida = InternalTypeId::of::<A>();
        let tidb = InternalTypeId::of::<B>();
        let tidc = InternalTypeId::of::<C<u32>>();
        let tidd = InternalTypeId::of::<C<f64>>();
        let tide = InternalTypeId::of::<C<A>>();
        let tidf = InternalTypeId::of::<C<B>>();
        let tidg = InternalTypeId::of::<C<C<A>>>();
        let tidh = InternalTypeId::of::<C<C<B>>>();

        // do it many times, even though 
        let iteration_count: usize = 100;
        let mut iterations = 0;
        while iterations < iteration_count {
            let tid0 = InternalTypeId::of::<A>();
            let tid1 = InternalTypeId::of::<B>();
            let tid2 = InternalTypeId::of::<C<u32>>();
            let tid3 = InternalTypeId::of::<C<f64>>();
            let tid4 = InternalTypeId::of::<C<A>>();
            let tid5 = InternalTypeId::of::<C<B>>();
            let tid6 = InternalTypeId::of::<C<C<A>>>();
            let tid7 = InternalTypeId::of::<C<C<B>>>();

            assert!(tid0 != tid1 && tid1 != tid2 && tid2 != tid3 && tid3 != tid4 && tid4 != tid5 && tid5 != tid6 && tid6 != tid7 && tid7 != tid0);
            assert!(tid0 == tida && tid1 == tidb && tid2 == tidc && tid3 == tidd && tid4 == tide && tid5 == tidf && tid6 == tidg && tid7 == tidh);
            
            assert_eq!(tida, InternalTypeId::of::<A>());
            assert_eq!(tidb, InternalTypeId::of::<B>());
            assert_eq!(tidc, InternalTypeId::of::<C<u32>>());
            assert_eq!(tidd, InternalTypeId::of::<C<f64>>());
            assert_eq!(tide, InternalTypeId::of::<C<A>>());
            assert_eq!(tidf, InternalTypeId::of::<C<B>>());
            assert_eq!(tidg, InternalTypeId::of::<C<C<A>>>());
            assert_eq!(tidh, InternalTypeId::of::<C<C<B>>>());

            assert_ne!(tida, InternalTypeId::of::<B>());
            assert_ne!(tidb, InternalTypeId::of::<A>());

            assert_ne!(tidg, InternalTypeId::of::<C<C<B>>>());
            assert_ne!(tidh, InternalTypeId::of::<C<C<A>>>());
   
            iterations += 1;
        }
    }
}
