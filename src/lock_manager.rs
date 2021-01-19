use std::{
    hash::Hash,
    sync::{Arc, Mutex},
};

use dashmap::DashMap;

#[derive(Debug)]
pub struct LockManager<K>(DashMap<K, Arc<Mutex<()>>>)
where
    K: Hash + Eq;

impl<K> LockManager<K>
where
    K: Hash + Eq,
{
    pub fn new() -> Self {
        Self(DashMap::new())
    }

    pub fn with_lock<T, F>(&self, key: K, f: F) -> T
    where
        F: FnOnce() -> T,
    {
        let lock = self
            .0
            .entry(key)
            .or_insert_with(|| Arc::new(Mutex::new(())))
            .clone();
        let _guard = lock.lock().unwrap();
        f()
    }
}
