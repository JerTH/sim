/// Collections

extern crate unsafe_any;

pub use unsafe_any::UnsafeAnyExt;
use std::{fmt::Debug, panic::AssertUnwindSafe};
use crate::{debug::MemoryUse, identity::EntityId};

pub enum TryReserveError {
    CapacityOverflow,
    AllocError,
}

pub(crate) trait Get<I> {
    type Item;
    fn get(&self, idx: I) -> Option<&Self::Item>;
}

pub(crate) trait GetMut<I> {
    type Item;
    fn get_mut(&mut self, idx: I) -> Option<&mut Self::Item>;
}

pub trait EntityIndexible {
    fn index_to_id(&self, idx: usize) -> Option<EntityId>;
}

#[derive(Debug, Clone)]
pub struct SparseSet<T> {
    sparse: Vec<usize>,
    dense: Vec<usize>,
    data: Vec<T>,
}

const EMPTY_KEY: usize = std::usize::MAX;

impl<T> SparseSet<T> {
    pub fn new() -> SparseSet<T> {
        SparseSet {
            sparse: Vec::new(),
            dense: Vec::new(),
            data: Vec::new(),
        }
    }
    
    /// Returns true if the `SparseSet` contains an item for `key`
    pub fn contains(&self, key: usize) -> bool {
        self.get_idx(key).is_some()
    }

    /// Inserts the item with the given key, if there is already a stored item associated with the key, returns Some(stored)
    /// 
    /// Returns None if there wasn't 
    pub fn insert_with(&mut self, key: usize, item: T) -> Option<T> {
        while key >= self.capacity() {
            let result = self.reserve( core::cmp::max(1usize,self.len()));
        }

        if let Some(stored) = self.get_mut(key) {
            return Some(std::mem::replace(stored, item))
        } else {
            self.sparse[key] = self.len();
            self.dense.push(key);
            self.data.push(item);
            return None
        }
    }
    
    /// Inserts an item into the SparseSet and returns the key in Ok(key) if successful, otherwise returns the inserted item in Err(item)
    pub fn insert(&mut self, item: T) -> Result<usize, T> {
        let mut key = self.sparse.len();
        if !self.is_empty() {
            for (idx, sparse) in self.sparse.iter().enumerate() {
                if *sparse == EMPTY_KEY {
                    key = idx;
                }
            }
        }

        if let Some(item) = self.insert_with(key, item) {
            return Err(item);
        } else {
            return Ok(key);
        }
    }
    
    pub fn remove(&mut self, key: usize) -> Option<T> {
        if let Some(idx) = self.get_idx(key) {
            let swap = *self.dense.last().unwrap();
            let (_, item) = (self.dense.swap_remove(idx), self.data.swap_remove(idx));
            
            if self.len() > 0 {
                self.sparse[swap] = idx;
            }

            // Set the sparse item to a marker value, for quicker testing of empty spaces
            self.sparse[key] = EMPTY_KEY;

            return Some(item)

        } else {
            return None
        }
    }
    
    /// Reserves space for `additional` elements in the SparseSet
    /// 
    /// If successful, returns Ok(new_capacity), otherwise returns an Err describing what went wrong
    /// 
    /// # Exception Safety
    /// 
    /// This method makes a best-effort attempt to be exception safe, internally it uses Vec::resize
    /// and Vec::reserve, which can potentially panic, this is wrapped in a check and a catch_unwind.
    /// 
    /// Despite this best-effort it is still possible for panic-aborts to be triggered inside the
    /// standard library, these cannot be caught, and will still result in program termination
    pub fn reserve(&mut self, additional: usize) -> Result<usize, TryReserveError> {
        let new_capacity = additional + self.capacity();

        if new_capacity < ::std::isize::MAX as usize {
            let old_sparse_len = self.sparse.len();

            // Closure
            //
            // UnwindSafe because we restore the internal state upon failure, thus there should
            // not be any externally observable inconsistent state
            let resize_and_reserve_unwind_safe = AssertUnwindSafe(|| {
                self.sparse.resize(new_capacity, EMPTY_KEY);
                self.dense.reserve(additional);
                self.data.reserve(additional);
            });

            match std::panic::catch_unwind(AssertUnwindSafe(resize_and_reserve_unwind_safe)) {
                Ok(_) => return Ok(new_capacity),
                Err(_) => {
                    // restore the state if it has changed
                    self.sparse.truncate(old_sparse_len);
                    self.dense.shrink_to_fit();
                    self.data.shrink_to_fit();
                    return Err(TryReserveError::AllocError);
                },
            }
        } else {
            return Err(TryReserveError::CapacityOverflow);
        }
    }

    pub fn shrink_to_fit(&mut self) {
        todo!()
    }
    
    pub fn capacity(&self) -> usize {
        self.sparse.len()
    }
    
    pub fn len(&self) -> usize {
        self.dense.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn clear(&mut self) {
        self.dense.clear();
    }

    pub fn get_key(&self, idx: usize) -> Option<usize> {
        self.dense.get(idx).map(|key| *key)
    }

    /// Gets the key/value pair for an item at a given raw index
    ///
    /// Safety:
    /// 
    /// SparseSet is unordered. Internally, items are free to move around, thus it's not generally useful to
    /// associate a raw index with a key/value pair. This function is declared unsafe to mitigate foot-gun usage.
    /// That said, this is still useful sometimes especially when iterating the contents of the SparseSet
    pub unsafe fn get_kv_pair(&self, idx: usize) -> Option<(usize, &T)> {
        // TODO: Revisit this, having an interface to directly get k/v pairs from internal indices isn't ideal
        //
        // Maybe use an iterator that returns k/v's? Investigate if this is even necessary at all
        //

        let key = self.dense.get(idx);
        let val = self.data.get(idx);
        
        if let (Some(k), Some(v)) = (key, val) {
            Some((*k, v))
        } else {
            None
        }
    }

    pub fn kv_pairs(&self) -> KeyValueIter<T> {
        KeyValueIter {
            set: self,
            idx: 0usize,
        }
    }

    pub fn iter(&self) -> <&Self as IntoIterator>::IntoIter {
        self.into_iter()
    }

    pub fn iter_mut(&mut self) -> <&mut Self as IntoIterator>::IntoIter {
        self.into_iter()
    }

    pub fn as_slice(&self) -> &[T] {
        self.data.as_slice()
    }
    
    pub fn as_mut_slice(&mut self) -> &mut [T] {
        self.data.as_mut_slice()
    }

    fn private_get(&self, key: usize) -> Option<&T> {
        if let Some(idx) = self.get_idx(key) {
            Some(&self.data[idx])
        } else {
            None
        }
    }

    fn private_get_mut(&mut self, key: usize) -> Option<&mut T> {
        if let Some(idx) = self.get_idx(key) {
            Some(&mut self.data[idx])
        } else {
            None
        }
    }

    fn get_idx(&self, key: usize) -> Option<usize> {
        if key >= self.capacity() {
            return None
        } else {
            let idx = self.sparse[key];

            if self.sparse[key] < self.len() {
                if self.dense[self.sparse[key]] == key {
                    return Some(idx)
                }
            } 
        }
        return None
    }
}

pub struct KeyValueIter<'a, T> {
    set: &'a SparseSet<T>,
    idx: usize,
}

impl<'a, T> Iterator for KeyValueIter<'a, T> {
    type Item = (usize, &'a T);

    fn next(&mut self) -> Option<Self::Item> {
        self.idx += 1;
        
        unsafe { self.set.get_kv_pair(self.idx - 1) }
    }
}

impl<T> MemoryUse for SparseSet<T> {
    fn memory_use_estimate(&self) -> usize {
        let mut total = std::mem::size_of_val(&self);
        total += self.sparse.capacity() * std::mem::size_of::<usize>();
        total += self.dense.capacity() * std::mem::size_of::<usize>();
        total += self.data.capacity() * std::mem::size_of::<T>();
        total
    }
}

impl<T> IntoIterator for SparseSet<T> {
    type Item = T;
    type IntoIter = std::vec::IntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        self.data.into_iter()
    }
}

impl<'a, T> IntoIterator for &'a SparseSet<T> {
    type Item = &'a T;
    type IntoIter = std::slice::Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        (&self.data).into_iter()
    }
}

impl<'a, T> IntoIterator for &'a mut SparseSet<T> {
    type Item = &'a mut T;
    type IntoIter = std::slice::IterMut<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        (&mut self.data).into_iter()
    }
}

impl<T> Get<usize> for SparseSet<T> {
    type Item = T;
    fn get(&self, idx: usize) -> Option<&Self::Item> {
        self.private_get(idx)
    }
}

impl<T> GetMut<usize> for SparseSet<T> {
    type Item = T;
    fn get_mut(&mut self, idx: usize) -> Option<&mut Self::Item> {
        self.private_get_mut(idx)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_sparse_set() {
        let n = 8;
        let mut set = SparseSet::new();

        fn to_letter(i: usize) -> char { (i + 97) as u8 as char }

        for i in 0..n {
            assert_eq!(None, set.insert_with(i as usize, to_letter(i)));
        }

        for i in 0..n {
            assert_eq!(&to_letter(i), set.get(i).unwrap());
        }

        for i in 0..n {
            assert_eq!(to_letter(i), set.remove(i).unwrap());

            for j in (i+1)..n {
                assert_eq!(&to_letter(j), set.get(j).unwrap());
            }
        }
    }
}
