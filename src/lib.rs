use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::RwLock;

pub trait Map<K: Hash + PartialEq, V: Copy> {
    fn get(&self, key: &K) -> Option<V>;
    fn contains(&self, key: &K) -> bool;
    fn put(&self, key: K, value: V);
    fn remove(&self, key: &K) -> bool;
}

pub struct StripedHashMap<K: Hash + PartialEq, V: Copy> {
    buckets: Vec<RwLock<Vec<(K, V)>>>,
}

impl<K: Hash + PartialEq, V: Copy> StripedHashMap<K, V> {
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

impl<K: Hash + PartialEq, V: Copy> Map<K, V> for StripedHashMap<K, V> {
    fn get(&self, key: &K) -> Option<V> {
        let hash = self.hash(&key);
        let bucket_idx = (hash as usize) % self.buckets.len();
        let bucket = self.buckets[bucket_idx].read().unwrap();
        for entry in bucket.iter() {
            if entry.0 == *key {
                return Some(entry.1)
            }
        }
        None
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
        let hash = self.hash(&key);
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
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
