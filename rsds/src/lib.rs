use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::ops::Deref;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::RwLock;
use std::sync::RwLockReadGuard;

pub trait Map<'a, K: Hash + PartialEq, V, VRef: 'a + Deref> {
    fn get(&'a self, key: &K) -> Option<VRef>;
    fn contains(&self, key: &K) -> bool;
    fn put(&self, key: K, value: V);
    fn remove(&self, key: &K) -> bool;
}

struct MaybeElemRef<'a, K: PartialEq, V> {
    guard: RwLockReadGuard<'a, Vec<(K, V)>>,
}

impl<'a, K: PartialEq, V> MaybeElemRef<'a, K, V> {
    fn find(self, key: &K) -> Option<ElemRef<'a, K, V>> {
        let itr = self.guard.iter();
        for (i, entry) in itr.enumerate() {
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
    bucket_sizes: Vec<AtomicUsize>,
    max_avg_bucket_size: usize,
    resize_in_progress: AtomicBool,
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
        const DEFAULT_MAX_AVG_BUCKET_SIZE: usize = 100;
        let buckets = (0..num_buckets).map(|_| RwLock::new(vec![])).collect();
        let bucket_sizes = (0..num_buckets).map(|_| AtomicUsize::new(0)).collect();
        StripedHashMap {
            buckets,
            bucket_sizes,
            max_avg_bucket_size: DEFAULT_MAX_AVG_BUCKET_SIZE,
            resize_in_progress: AtomicBool::new(false),
        }
    }

    fn hash(&self, key: &K) -> usize {
        let mut hasher = DefaultHasher::new();
        key.hash(&mut hasher);
        hasher.finish() as usize
    }

    fn _should_resize(&self) -> bool {
        self._avg_bucket_size_relaxed() >= self.max_avg_bucket_size
    }

    fn _avg_bucket_size_relaxed(&self) -> usize {
        self._avg_bucket_size(Ordering::Relaxed)
    }

    fn _avg_bucket_size(&self, ordering: Ordering) -> usize {
        let num_buckets = self.buckets.len();
        let bucket_sz_sum = self
            .bucket_sizes
            .iter()
            .fold(0, |acc, cur| acc + cur.load(ordering));
        bucket_sz_sum / num_buckets
    }

    fn _resize(&self) {}
}

impl<'a, K: Hash + PartialEq, V> Map<'a, K, V, ElemRef<'a, K, V>> for StripedHashMap<K, V> {
    fn get(&'a self, key: &K) -> Option<ElemRef<'a, K, V>> {
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
        self.bucket_sizes[bucket_idx].fetch_add(1, Ordering::Relaxed);

        if self._should_resize() {
            if self
                .resize_in_progress
                .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
                .is_ok()
            {
                self._resize();
                self.resize_in_progress.swap(false, Ordering::SeqCst);
            }
        }
    }

    fn remove(&self, key: &K) -> bool {
        let hash = self.hash(key);
        let bucket_idx = (hash as usize) % self.buckets.len();
        let mut bucket = self.buckets[bucket_idx].write().unwrap();
        let itr = bucket.iter();
        for (i, entry) in itr.enumerate() {
            if entry.0 == *key {
                bucket.remove(i);
                self.bucket_sizes[bucket_idx].fetch_sub(1, Ordering::Relaxed);
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
