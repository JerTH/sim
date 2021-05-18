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

pub struct WorldSystem {
    name: String,
    func: Box<dyn Fn(LocalWorld) -> Result<(), WorldSystemError>>,
    id: SystemId,

    // a local execution id, superceded by `id`
    #[deprecated] exec_id: SystemExecutionId,

    // dependencies represented as id's, only used for conflict resolution
    reads: Vec<ComponentSetId>,
    writes: Vec<ComponentSetId>,
}

impl WorldSystem {
    fn run(&self, local_world: LocalWorld) -> Result<(), WorldSystemError> {
        (self.func)(local_world)
    }

    #[deprecated]
    fn execution_id(&self) -> SystemExecutionId {
        self.exec_id
    }

    pub(crate) fn id(&self) -> SystemId {
        self.id
    }
    
    pub(crate) fn set_id(&mut self, id: SystemId) {
        self.id = id;
    }

    // /fn mark_dependency(&mut self, dependency: DependencyType, component_id: ComponentSetId) {
    // /    todo!()
    // /}

    #[deprecated]
    pub(crate) fn mark_write_dependency<T>(&mut self) where T: Component {
        self.writes.push(ComponentSetId::of::<T>())
    }

    #[deprecated]
    pub(crate) fn mark_read_dependency<T>(&mut self) where T: Component {
        self.reads.push(ComponentSetId::of::<T>())
    }
}

impl Display for WorldSystem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = self.name.as_str();
        write!(f, "{}", name)
    }
}

impl ConflictCmp for &WorldSystem {
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

impl Debug for WorldSystem {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("WorldSystem")
        .field("func", &"dyn Fn(&LocalWorld) -> SystemResult")
        .field("reads", &self.reads)
        .field("writes", &self.writes)
        .field("name", &self.name)
        .finish()
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
