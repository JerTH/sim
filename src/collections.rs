/// Collections

pub struct SparseSet<T> {
    sparse: Vec<usize>,
    dense: Vec<usize>,
    data: Vec<T>,
}

impl<T> SparseSet<T> {
    pub fn new() -> SparseSet<T> {
        SparseSet {
            sparse: Vec::new(),
            dense: Vec::new(),
            data: Vec::new(),
        }
    }

    pub fn get(&self, key: usize) -> Option<&T> {
        if let Some(idx) = self.get_idx(key) {
            Some(&self.data[idx])
        } else {
            None
        }
    }

    pub fn get_mut(&mut self, key: usize) -> Option<&mut T> {
        if let Some(idx) = self.get_idx(key) {
            Some(&mut self.data[idx])
        } else {
            None
        }
    }

    pub fn contains(&self, key: usize) -> bool {
        self.get_idx(key).is_some()
    }

    pub fn insert(&mut self, key: usize, item: T) -> Option<T> {
        while key > self.capacity() {
            self.reserve( core::cmp::max(1usize,self.len() * 2usize));
        }

        if let Some(stored) = self.get_mut(key) {
            return Some(std::mem::replace(stored, item))
        } else {
            let n = self.len();
            self.dense.push(key);
            self.data.push(item);
            self.sparse[key] = n;

            return None
        }
    }
    
    pub fn remove(&mut self, key: usize) -> Option<T> {
        if let Some(idx) = self.get_idx(key) {
            assert_eq!(key, self.dense.swap_remove(idx));

            let item = self.data.swap_remove(idx);
            
            if !self.is_empty() {
                let swapped_key = self.dense[idx];
                self.sparse[swapped_key] = idx;
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
        if key > self.capacity() {
            return None
        } else {
            let idx = self.sparse[key];
            if idx < self.len() && self.dense[idx] == key {
                return Some(idx)
            } 
        }
        return None
    }
}
