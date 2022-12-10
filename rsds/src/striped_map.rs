use crate::Map;
use crossbeam::utils::CachePadded;
use std::collections::hash_map::RandomState;
use std::hash::{BuildHasher, Hash, Hasher};
use std::ops::Deref;
use std::sync::atomic::{AtomicBool, AtomicPtr, AtomicUsize, Ordering};
use std::sync::Arc;
use std::sync::RwLock;
use std::sync::RwLockReadGuard;
use std::sync::RwLockWriteGuard;

const DEFAULT_NUM_BUCKETS: usize = 10;
const DEFAULT_MAX_AVG_BUCKET_SIZE: usize = 500;

type Bucket<K, V> = Vec<(K, V)>;

type ProtectedBucket<K, V> = RwLock<Bucket<K, V>>;

struct MaybeElemRef<'a, K: PartialEq, V> {
    guard: RwLockReadGuard<'a, Bucket<K, V>>,
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
    guard: RwLockReadGuard<'a, Bucket<K, V>>,
}

impl<'a, K: PartialEq, V> Deref for ElemRef<'a, K, V> {
    type Target = V;

    fn deref(&self) -> &Self::Target {
        &self.guard[self.idx].1
    }
}

pub struct StripedHashMap<K: Hash + PartialEq, V, S = RandomState> {
    buckets: CachePadded<AtomicPtr<Vec<ProtectedBucket<K, V>>>>,
    bucket_sizes: CachePadded<AtomicPtr<Arc<Vec<CachePadded<AtomicUsize>>>>>,
    max_avg_bucket_size: usize,
    resize_in_progress: CachePadded<AtomicBool>,
    state: S,
}

impl<K, V> Default for StripedHashMap<K, V, RandomState>
where
    K: Hash + PartialEq,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<K, V> StripedHashMap<K, V, RandomState>
where
    K: Hash + PartialEq,
{
    pub fn new() -> Self {
        StripedHashMap::build(DEFAULT_NUM_BUCKETS, RandomState::default())
    }

    pub fn with_num_buckets(num_buckets: usize) -> Self {
        StripedHashMap::build(num_buckets, RandomState::default())
    }
}

impl<K, V, S> StripedHashMap<K, V, S>
where
    K: Hash + PartialEq,
    S: BuildHasher,
{
    pub fn with_hasher(hasher: S) -> Self {
        StripedHashMap::build(DEFAULT_NUM_BUCKETS, hasher)
    }

    fn build(num_buckets: usize, hasher: S) -> Self {
        let buckets: Vec<ProtectedBucket<K, V>> =
            (0..num_buckets).map(|_| RwLock::new(vec![])).collect();

        let wrapped_buckets = Box::new(buckets);
        let bucket_ptr = Box::into_raw(wrapped_buckets);

        let bucket_sizes = Box::new(Arc::new(
            (0..num_buckets)
                .map(|_| CachePadded::new(AtomicUsize::new(0)))
                .collect(),
        ));
        let bucket_sizes_ptr = Box::into_raw(bucket_sizes);

        StripedHashMap {
            buckets: CachePadded::new(AtomicPtr::new(bucket_ptr)),
            bucket_sizes: CachePadded::new(AtomicPtr::new(bucket_sizes_ptr)),
            max_avg_bucket_size: DEFAULT_MAX_AVG_BUCKET_SIZE,
            resize_in_progress: CachePadded::new(AtomicBool::new(false)),
            state: hasher,
        }
    }
}

impl<K, V, S> StripedHashMap<K, V, S>
where
    K: Hash + PartialEq,
    S: BuildHasher,
{
    fn hash(&self, key: &K) -> usize {
        let mut hasher = self.state.build_hasher();
        key.hash(&mut hasher);
        hasher.finish() as usize
    }

    fn num_buckets(&self) -> usize {
        unsafe { (*self.buckets.load(Ordering::SeqCst)).len() }
    }

    fn _get_read_bucket_by_key(&self, key: &K) -> RwLockReadGuard<Bucket<K, V>> {
        let hash = self.hash(key);
        loop {
            self._guard_resize();
            let buckets = unsafe { &*self.buckets.load(Ordering::SeqCst) };
            if self.resize_in_progress.load(Ordering::SeqCst) {
                continue;
            }
            let bucket_index = hash % buckets.len();
            let r = buckets[bucket_index].read().unwrap();
            if self.resize_in_progress.load(Ordering::SeqCst) {
                drop(r);
                continue;
            }
            return r;
        }
    }

    fn _get_write_bucket_by_key(&self, key: &K) -> (usize, RwLockWriteGuard<Bucket<K, V>>) {
        let hash = self.hash(key);
        loop {
            self._guard_resize();
            let buckets = unsafe { &*self.buckets.load(Ordering::SeqCst) };
            if self.resize_in_progress.load(Ordering::SeqCst) {
                continue;
            }
            let bucket_index = hash % buckets.len();
            let w = buckets[bucket_index].write().unwrap();
            if self.resize_in_progress.load(Ordering::SeqCst) {
                drop(w);
                continue;
            }
            return (bucket_index, w);
        }
    }

    fn _should_resize(&self) -> bool {
        self._avg_bucket_size_relaxed() >= self.max_avg_bucket_size
    }

    fn _avg_bucket_size_relaxed(&self) -> usize {
        self._avg_bucket_size(Ordering::Relaxed)
    }

    fn _avg_bucket_size(&self, ordering: Ordering) -> usize {
        let bucket_sizes = unsafe { &*self.bucket_sizes.load(Ordering::SeqCst) };
        let bucket_sz_sum = bucket_sizes
            .iter()
            .fold(0, |acc, cur| acc + cur.load(ordering));
        bucket_sz_sum / self.num_buckets()
    }

    fn _increment_bucket_size(&self, bucket_index: usize) {
        let bucket_sizes = unsafe { &*self.bucket_sizes.load(Ordering::SeqCst) };
        bucket_sizes[bucket_index].fetch_add(1, Ordering::Relaxed);
    }

    fn _decrement_bucket_size(&self, bucket_index: usize) {
        let bucket_sizes = unsafe { &*self.bucket_sizes.load(Ordering::SeqCst) };
        bucket_sizes[bucket_index].fetch_sub(1, Ordering::Relaxed);
    }

    fn _resize(&self) {
        let buckets = unsafe { Box::from_raw(self.buckets.load(Ordering::SeqCst)) };
        let old_len = buckets.len();
        let new_len = old_len * 2;
        let mut new_buckets: Vec<Bucket<K, V>> = (0..new_len).map(|_| Vec::new()).collect();

        // flush out all pending readers/writers.
        // this allows us to safely move data from the old buckets to the new.
        for bucket in buckets.iter() {
            drop(bucket.write().unwrap());
        }

        for locked_bucket in buckets.into_iter() {
            let bucket = locked_bucket.into_inner().unwrap();
            for (k, v) in bucket {
                let hash = self.hash(&k);
                let new_bucket_idx = hash % new_len;
                new_buckets[new_bucket_idx].push((k, v));
            }
        }

        let new_bucket_sizes = new_buckets
            .iter()
            .map(|b| CachePadded::new(AtomicUsize::new(b.len())))
            .collect();

        let new_buckets_locked = new_buckets.into_iter().map(RwLock::new).collect();
        let new_buckets_wrapped = Box::new(new_buckets_locked);
        let new_buckets_ptr = Box::into_raw(new_buckets_wrapped);
        self.buckets.swap(new_buckets_ptr, Ordering::SeqCst);

        let new_bucket_sizes_ptr = Box::into_raw(Box::new(Arc::new(new_bucket_sizes)));
        self.bucket_sizes
            .swap(new_bucket_sizes_ptr, Ordering::SeqCst);
    }

    fn _guard_resize(&self) {
        while self.resize_in_progress.load(Ordering::SeqCst) {
            std::hint::spin_loop()
        }
    }
}

impl<K, V, S> Drop for StripedHashMap<K, V, S>
where
    K: Hash + PartialEq,
{
    fn drop(&mut self) {
        let buckets_ptr = self.buckets.load(Ordering::SeqCst);
        let buckets = unsafe { Box::from_raw(buckets_ptr) };
        drop(buckets);
    }
}

impl<'a, K, V, S> Map<'a, K, V, ElemRef<'a, K, V>> for StripedHashMap<K, V, S>
where
    K: Hash + PartialEq,
    S: BuildHasher,
{
    fn get(&'a self, key: &K) -> Option<ElemRef<'a, K, V>> {
        let searcher = MaybeElemRef {
            guard: self._get_read_bucket_by_key(key),
        };
        searcher.find(key)
    }

    fn contains(&self, key: &K) -> bool {
        self.get(key).is_some()
    }

    fn put(&self, key: K, value: V) {
        let (bucket_idx, mut bucket) = self._get_write_bucket_by_key(&key);
        bucket.push((key, value));
        self._increment_bucket_size(bucket_idx);

        #[allow(clippy::collapsible_if)]
        if self._should_resize() {
            if self
                .resize_in_progress
                .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
                .is_ok()
            {
                drop(bucket);
                self._resize();
                self.resize_in_progress.swap(false, Ordering::SeqCst);
            }
        }
    }

    fn remove(&self, key: &K) -> bool {
        let (bucket_idx, mut bucket) = self._get_write_bucket_by_key(key);
        let itr = bucket.iter();
        for (i, entry) in itr.enumerate() {
            if entry.0 == *key {
                bucket.remove(i);
                self._decrement_bucket_size(bucket_idx);
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
