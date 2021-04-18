/// Collections

extern crate unsafe_any;

pub use unsafe_any::UnsafeAnyExt;
use std::{any::Any, fmt::Debug};
use crate::{debug::MemoryUse, identity::EntityId};

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

impl<T: Debug> SparseSet<T> {
    pub fn new() -> SparseSet<T> {
        SparseSet {
            sparse: Vec::new(),
            dense: Vec::new(),
            data: Vec::new(),
        }
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

    pub fn contains(&self, key: usize) -> bool {
        self.get_idx(key).is_some()
    }
    
    // When an item is inserted with a key:
    // dense.last() == key
    // data.last() == item
    // sparse[key] == data.last()
    pub fn insert(&mut self, key: usize, item: T) -> Option<T> {
        while key >= self.capacity() {
            self.reserve( core::cmp::max(1usize,self.len()));
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
    
    pub fn remove(&mut self, key: usize) -> Option<T> {
        if let Some(idx) = self.get_idx(key) {
            let swap = *self.dense.last().unwrap();
            let (_, item) = (self.dense.swap_remove(idx), self.data.swap_remove(idx));
            
            if self.len() > 0 {
                self.sparse[swap] = idx;
            }

            self.sparse[key] = self.capacity();

            return Some(item)

        } else {
            return None
        }
    }
    
    pub fn reserve(&mut self, additional: usize) {
        let new_capacity = additional + self.capacity();
        self.sparse.resize(new_capacity, new_capacity);
        self.dense.reserve(additional);
        self.data.reserve(additional);
    }

    pub fn shrink_to_fit(&mut self) {
        unimplemented!()
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

    pub fn get_key(&self, idx: usize) -> Option<usize> {
        self.dense.get(idx).map(|key| *key)
    }
    
    /// Gets the key/value pair for an item at a given raw index
    ///
    /// Safety:
    /// 
    /// SparseSet is unordered. Internally, items are free to move around, thus it's not generally useful to
    /// associate a raw index with a key/value pair. This function is declared unsafe to mitigate
    /// possible foot-gun usage. That said, this is still useful sometimes especially when iterating the
    /// contents of the SparseSet
    pub unsafe fn get_kv(&self, idx: usize) -> Option<(usize, &T)> {
        let key = self.dense.get(idx);
        let val = self.data.get(idx);
        
        if let (Some(k), Some(v)) = (key, val) {
            Some((*k, v))
        } else {
            None
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

impl<T: Debug> Get<usize> for SparseSet<T> {
    type Item = T;
    fn get(&self, idx: usize) -> Option<&Self::Item> {
        self.private_get(idx)
    }
}

impl<T: Debug> GetMut<usize> for SparseSet<T> {
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
            assert_eq!(None, set.insert(i as usize, to_letter(i)));
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
