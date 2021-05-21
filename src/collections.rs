/// Collections

pub use unsafe_any::UnsafeAnyExt;
use std::{fmt::Debug, marker::PhantomData, panic::AssertUnwindSafe};

trait SparseSetKey: From<usize> + Clone + Copy {}

const EMPTY_KEY: usize = std::usize::MAX;

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

/// SparseSet
#[derive(Debug, Clone)]
pub struct SparseSet<T, K = usize> where K: Into<usize> + Clone + Copy {
    // TODO: - Large key optimization. Currently, if the user inserts with a large key, memory is allocated
    //         proportional to the key size. This makes SparseSet great for keys in the range close to 0, but
    //         very bad for random keys, or keys that are much larger than 0, it's very easy to trigger an out
    //         of memory condition by accident if one inserts with a random key. To fix this, large keys should be
    //         handled as a special case. They won't enjoy the same benefits as small keys that make SparseSet
    //         valuable, but fix this will insulate against mis-use and improve soundness

    sparse: Vec<usize>,
    dense: Vec<usize>,
    data: Vec<T>,
    _key: PhantomData<K>,
}

impl<T, K> SparseSet<T, K> where K: Into<usize> + Clone + Copy {
    pub fn new() -> SparseSet<T, K> {
        SparseSet {
            sparse: Vec::new(),
            dense: Vec::new(),
            data: Vec::new(),
            _key: PhantomData,
        }
    }

    /// Inserts an item into the SparseSet and returns the key in Ok(key) if successful, otherwise returns the inserted item in Err(item)
    ///
    /// NOTE:
    /// 
    /// Where `insert_with` requires the weaker `K: Into<usize>`, `insert` requires `K: From<usize>`, this is because `insert` generates
    /// a new key, whereas `insert_with` uses a provided key. In this way `SparseSet` can still be used with key types that are created
    /// in special ways by the user
    pub fn insert(&mut self, item: T) -> K where K: From<usize> {        
        let key: K = self.next_key();

        match self.insert_with(key, item) {
            Some(_) => {
                panic!("SparseSet::insert expected an empty index"); // it's a bug if we get an item back here
            },
            _ => ()
        }
        return key;
    }
    
    /// Inserts the item with the given key, if there is already a stored item associated with the key, returns Some(stored)
    /// 
    /// Returns None if there wasn't 
    pub fn insert_with(&mut self, key: K, item: T) -> Option<T> where K: Into<usize> {
        while key.into() >= self.capacity() {
            let _result = self.reserve( core::cmp::max(1usize,self.len()));
        }

        if let Some(stored) = self.get_mut(key) {
            return Some(std::mem::replace(stored, item))
        } else {
            self.sparse[key.into()] = self.len();
            self.dense.push(key.into());
            self.data.push(item);
            return None
        }
    }
    
    pub fn remove(&mut self, key: K) -> Option<T> where K: Into<usize>  {
        if let Some(idx) = self.get_idx(key) {
            let swap = *self.dense.last().unwrap();
            let (_, item) = (self.dense.swap_remove(idx), self.data.swap_remove(idx));
            
            if self.len() > 0 {
                self.sparse[swap] = idx;
            }

            // Set the sparse item to a marker value, for quicker testing of empty spaces
            self.sparse[key.into()] = EMPTY_KEY;

            return Some(item)

        } else {
            return None
        }
    }

    /// Returns the next free key that would be used by `insert`
    /// 
    /// Useful in conjunction with `insert_with`
    pub fn next_key(&self) -> K where K: From<usize>  {
        if self.is_empty() {
            return self.sparse.len().into(); // this is the first and only key
        } else {
            for (idx, sparse) in self.sparse.iter().enumerate() {
                if *sparse == EMPTY_KEY {
                    return K::from(idx);
                }
            }
            return self.sparse.len().into(); // no unused keys
        }
    }

    /// Returns true if the `SparseSet` contains an item for `key`
    pub fn contains(&self, key: K) -> bool {
        self.get_idx(key).is_some()
    }
    
    /// Reserves space for `additional` elements in the SparseSet
    /// 
    /// If successful, returns Ok(new_capacity), otherwise returns an Err describing what went wrong
    /// 
    /// # Exception Safety
    /// 
    /// This method makes a best-effort attempt to be exception safe, internally it uses Vec::resize
    /// and Vec::reserve, which can potentially panic, this is wrapped in a check and a catch_unwind
    /// and instead an error is returned.
    /// 
    /// Despite this best-effort it is still possible for panic-aborts to be triggered inside the
    /// standard library, these cannot be caught, and will still result in program termination.
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
    /// associate a the raw index with passed into this function with a key/value pair. This function is declared
    /// unsafe to mitigate foot-gun usage. With that said, when used with care this is still sometimes useful
    /// especially when iterating over the contents of the SparseSet
    pub unsafe fn get_kv_pair(&self, idx: usize) -> Option<(usize, &T)> {

        let key = self.dense.get(idx);
        let val = self.data.get(idx);
        
        if let (Some(k), Some(v)) = (key, val) {
            Some((*k, v))
        } else {
            None
        }
    }

    pub fn kv_pairs(&self) -> KeyValueIter<T, K> {
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

    fn _private_get(&self, key: K) -> Option<&T> {
        if let Some(idx) = self.get_idx(key) {
            Some(&self.data[idx])
        } else {
            None
        }
    }
    
    fn _private_get_mut(&mut self, key: K) -> Option<&mut T> where K: Into<usize> {
        if let Some(idx) = self.get_idx(key) {
            Some(&mut self.data[idx])
        } else {
            None
        }
    }

    fn get_idx(&self, key: K) -> Option<usize> {
        let key: usize = key.into();

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

pub struct KeyValueIter<'a, T, K = usize> where K: Into<usize> + Clone + Copy {
    set: &'a SparseSet<T, K>,
    idx: usize,
}

impl<'a, T, K> Iterator for KeyValueIter<'a, T, K> where K: Into<usize> + Clone + Copy {
    type Item = (usize, &'a T);

    fn next(&mut self) -> Option<Self::Item> {
        self.idx += 1;
        
        unsafe { self.set.get_kv_pair(self.idx - 1) }
    }
}

impl<T, K> IntoIterator for SparseSet<T, K> where K: Into<usize> + Clone + Copy {
    type Item = T;
    type IntoIter = std::vec::IntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        self.data.into_iter()
    }
}

impl<'a, T, K> IntoIterator for &'a SparseSet<T, K> where K: Into<usize> + Clone + Copy {
    type Item = &'a T;
    type IntoIter = std::slice::Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        (&self.data).into_iter()
    }
}

impl<'a, T, K> IntoIterator for &'a mut SparseSet<T, K> where K: Into<usize> + Clone + Copy{
    type Item = &'a mut T;
    type IntoIter = std::slice::IterMut<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        (&mut self.data).into_iter()
    }
}

// owned key
impl<T, K> Get<K> for SparseSet<T, K> where K: Into<usize> + Clone + Copy{
    type Item = T;
    fn get(&self, idx: K) -> Option<&Self::Item> {
        self._private_get(idx)
    }
}

impl<T, K> GetMut<K> for SparseSet<T, K> where K: Into<usize> + Clone + Copy{
    type Item = T;
    fn get_mut(&mut self, idx: K) -> Option<&mut Self::Item> {
        self._private_get_mut(idx)
    }
}

// reference to a key
impl<T, K> Get<&K> for SparseSet<T, K> where K: Into<usize> + Clone + Copy{
    type Item = T;
    fn get(&self, idx: &K) -> Option<&Self::Item> {
        self._private_get(*idx)
    }
}

impl<T, K> GetMut<&K> for SparseSet<T, K> where K: Into<usize> + Clone + Copy{
    type Item = T;
    fn get_mut(&mut self, idx: &K) -> Option<&mut Self::Item> {
        self._private_get_mut(*idx)
    }
}

struct Key(u32);

/// A generational set implemented over SparseSet
/// 
/// Each item stored in the set has a corresponding generation. Generations track the uniqueness of re-used indices
/// 
/// When an item is "deleted" from the set, its generation is incremented, subsequent access to the item first checks
/// the generation to ensure we never access stale data
struct GenerationalSparseSet<T, K = Key, G = usize, S = usize>
where
    K: From<(G, S)> + Into<(G, S)>,
    G: From<usize> + Copy + PartialEq,
    S: From<usize> + Into<usize> + Copy,
{
    free: Vec<S>,
    inner: SparseSet<(G, T), S>,
    _key: PhantomData<K>,
}

impl<T, K, G, S> GenerationalSparseSet<T, K, G, S>
where
    K: From<(G, S)> + Into<(G, S)>,
    G: From<usize> + Copy + PartialEq,
    S: From<usize> + Into<usize> + Copy,
{
    fn insert(item: T) -> Result<K, T> {
        Err(item)
    }

    fn insert_with(key: K, item: T) -> Result<(), T> {
        Err(item)
    }

    fn remove(key: K) -> Result<T, ()> {
        Err(())
    }
}

impl<T, K, G, S> Get<K> for GenerationalSparseSet<T, K, G, S>
where
    K: From<(G, S)> + Into<(G, S)>,
    G: From<usize> + Copy + PartialEq,
    S: From<usize> + Into<usize> + Copy,
{
    type Item = T;

    fn get(&self, key: K) -> Option<&Self::Item> {
        let (gen, idx) = K::into(key);

        if let Some(item) = self.inner.get(idx) {
            if item.0 == gen {
                return Some(&item.1)
            }
        }

        None
    }
}

impl<T, K, G, S> GetMut<K> for GenerationalSparseSet<T, K, G, S>
where
    K: From<(G, S)> + Into<(G, S)>,
    G: From<usize> + Copy + PartialEq,
    S: From<usize> + Into<usize> + Copy,
{
    type Item = T;

    fn get_mut(&mut self, key: K) -> Option<&mut Self::Item> {
        let (gen, idx) = K::into(key);

        if let Some(item) = self.inner.get_mut(idx) {
            if item.0 == gen {
                return Some(&mut item.1)
            }
        }

        None
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
