use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::ops::Deref;
use std::sync::RwLock;
use std::sync::RwLockReadGuard;

pub trait Map<K: Hash + PartialEq, V> {
    fn get(&self, key: &K) -> Option<ElemRef<K, V>>;
    fn contains(&self, key: &K) -> bool;
    fn put(&self, key: K, value: V);
    fn remove(&self, key: &K) -> bool;
}

struct MaybeElemRef<'a, K: PartialEq, V> {
    guard: RwLockReadGuard<'a, Vec<(K, V)>>,
}

impl<'a, K: PartialEq, V> MaybeElemRef<'a, K, V> {
    fn find(self, key: &K) -> Option<ElemRef<'a, K, V>> {
        for (i, entry) in self.guard.iter().enumerate() {
            if entry.0 == *key {
                return Some(ElemRef {
                    idx: i,
                    guard: self.guard,
                });
            }
        }
        None
    }
}

pub struct ElemRef<'a, K: PartialEq, V> {
    idx: usize,
    guard: RwLockReadGuard<'a, Vec<(K, V)>>,
}

impl<'a, K: PartialEq, V> Deref for ElemRef<'a, K, V> {
    type Target = V;

    fn deref(&self) -> &Self::Target {
        &self.guard[self.idx].1
    }
}

pub struct StripedHashMap<K: Hash + PartialEq, V> {
    buckets: Vec<RwLock<Vec<(K, V)>>>,
}

impl<K: Hash + PartialEq, V> Default for StripedHashMap<K, V> {
    fn default() -> Self {
        Self::new()
    }
}

impl<K: Hash + PartialEq, V> StripedHashMap<K, V> {
    pub fn new() -> Self {
        const DEFAULT_NUM_BUCKETS: usize = 10;
        StripedHashMap::with_num_buckets(DEFAULT_NUM_BUCKETS)
    }

    pub fn with_num_buckets(num_buckets: usize) -> Self {
        let buckets = (0..num_buckets).map(|_| RwLock::new(vec![])).collect();
        StripedHashMap { buckets }
    }

    fn hash(&self, key: &K) -> usize {
        let mut hasher = DefaultHasher::new();
        key.hash(&mut hasher);
        hasher.finish() as usize
    }
}

impl<K: Hash + PartialEq, V> Map<K, V> for StripedHashMap<K, V> {
    fn get(&self, key: &K) -> Option<ElemRef<K, V>> {
        let hash = self.hash(key);
        let bucket_idx = (hash as usize) % self.buckets.len();
        let bucket = self.buckets[bucket_idx].read().unwrap();
        let searcher = MaybeElemRef { guard: bucket };
        searcher.find(key)
    }

    fn contains(&self, key: &K) -> bool {
        self.get(key).is_some()
    }

    fn put(&self, key: K, value: V) {
        let hash = self.hash(&key);
        let bucket_idx = (hash as usize) % self.buckets.len();
        let mut bucket = self.buckets[bucket_idx].write().unwrap();
        bucket.push((key, value));
    }

    fn remove(&self, key: &K) -> bool {
        let hash = self.hash(key);
        let bucket_idx = (hash as usize) % self.buckets.len();
        let mut bucket = self.buckets[bucket_idx].write().unwrap();
        for (i, entry) in bucket.iter().enumerate() {
            if entry.0 == *key {
                bucket.remove(i);
                return true;
            }
        }

        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hashtable() {
        let map = StripedHashMap::new();
        let key = "hello".to_string();
        let val = "world".to_string();
        map.put(key.clone(), val.clone());
        assert!(map.contains(&key));
        assert_eq!(*map.get(&key).unwrap(), val);
    }
}
