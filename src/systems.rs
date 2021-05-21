use std::{error::Error, fmt::{Debug, Display}};

use crate::{collections::{Get, GetMut, SparseSet}, components::{Component, ComponentSetId}, conflictgraph::{ConflictCmp, ConflictGraphError}, identity::{LinearId, SystemExecutionId}, world::{LocalWorld}};

#[derive(Debug)]
pub enum WorldSystemError {
    FailedToResolveSystemTree(ConflictGraphError),
    FailedToAddWorldSystem(WorldSystem),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SystemId(usize);

impl From<usize> for SystemId {
    fn from(id: usize) -> Self {
        SystemId(id)
    }
}

impl From<SystemId> for usize {
    fn from(id: SystemId) -> Self {
        id.0
    }
}

impl Display for WorldSystemError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::FailedToResolveSystemTree(inner) => write!(f, "failed to resolve system tree: {}", inner),
            Self::FailedToAddWorldSystem(system) => write!(f, "failed to add world system: {}", system),
        }
    }
}

impl Error for WorldSystemError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::FailedToResolveSystemTree(inner) => Some(inner),
            Self::FailedToAddWorldSystem(_) => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DependencyType {
    Read,
    Write,
}

pub type WorldSystemFn = fn(LocalWorld) -> Result<(), WorldSystemError>;
pub trait WorldSystemFnTrait: Fn(LocalWorld) -> Result<(), WorldSystemError> {}

pub struct WorldSystem {
    name: String,
    id: SystemId,

    // the system code executed every engine loop
    system_fn: WorldSystemFn,

    // dependencies represented as id's, only used for conflict resolution
    reads: Vec<ComponentSetId>,
    writes: Vec<ComponentSetId>,
}

impl WorldSystem {
    pub(crate) fn new(system_fn: WorldSystemFn) -> Self {
        WorldSystem {
            name: String::from(""),
            id: SystemId::from(0),
            system_fn: system_fn,
            reads: Vec::new(),
            writes: Vec::new(),
        }
    }

    pub(crate) fn run(&self, local_world: LocalWorld) -> Result<(), WorldSystemError> {
        (self.system_fn)(local_world)
    }

    pub(crate) fn id(&self) -> SystemId {
        self.id
    }
    
    pub(crate) fn set_id(&mut self, id: SystemId) {
        self.id = id;
    }
}

impl<'a> ConflictCmp for &WorldSystem {
    fn conflict_cmp(&self, other: &Self) -> bool {
        // conflict if two systems write the same data or one node reads and the other node writes

        for read in self.reads.iter() {
            for write in other.writes.iter() {
                if read == write {
                    return true
                }
            }
        }
        
        for write in self.writes.iter() {
            for read in other.reads.iter() {
                if write == read {
                    return true
                }
            }
            
            for other_write in other.writes.iter() {
                if write == other_write {
                    return true
                }
            }
        }
        return false
    }
}

impl<'a> Debug for WorldSystem {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("WorldSystem")
        .field("func", &"dyn Fn(&LocalWorld) -> SystemResult")
        .field("reads", &self.reads)
        .field("writes", &self.writes)
        .field("name", &self.name)
        .finish()
    }
}

impl<'a> Display for WorldSystem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = self.name.as_str();
        write!(f, "{}", name)
    }
}


/// A group of systems and their dependencies
#[derive(Debug, Default)]
pub(crate) struct SystemGroup {
    systems: Vec<SystemId>,
    group_mutable: Vec<ComponentSetId>,
    group_immutable: Vec<ComponentSetId>,
}

impl SystemGroup {
    pub fn system_ids(&self) -> &Vec<SystemId> {
        &self.systems
    }

    pub fn group_immutable(&self) -> &Vec<ComponentSetId> {
        &self.group_immutable
    }

    pub fn group_mutable(&self) -> &Vec<ComponentSetId> {
        &self.group_mutable
    }
}

impl From<Vec<&WorldSystem>> for SystemGroup {
    fn from(systems: Vec<&WorldSystem>) -> Self {
        let mut group = SystemGroup::default();

        for system in systems {
            group.systems.push(system.id());
            group.group_immutable.extend(&system.reads);
            group.group_mutable.extend(&system.writes);
        }

        group.group_immutable.sort();
        group.group_immutable.dedup();
        
        group.group_mutable.sort();
        group.group_mutable.dedup();

        return group;
    }
}


//impl Get<SystemId> for SparseSet<WorldSystem> {
//    type Item = WorldSystem;
//    fn get(&self, idx: SystemId) -> Option<&Self::Item> {
//        self.get(idx)
//    }
//}
//
//impl GetMut<SystemId> for SparseSet<WorldSystem> {
//    type Item = WorldSystem;
//    fn get_mut(&mut self, idx: SystemId) -> Option<&mut Self::Item> {
//        self.get_mut(idx)
//    }
//}

//impl Get<&SystemId> for SparseSet<WorldSystem> {
//    type Item = WorldSystem;
//    fn get(&self, idx: &SystemId) -> Option<&Self::Item> {
//        self.get(idx.as_linear_u64() as usize)
//    }
//}
//
//impl GetMut<&SystemId> for SparseSet<WorldSystem> {
//    type Item = WorldSystem;
//    fn get_mut(&mut self, idx: &SystemId) -> Option<&mut Self::Item> {
//        self.get_mut(idx.as_linear_u64() as usize)
//    }
//}
