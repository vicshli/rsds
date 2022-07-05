mod striped_map;

pub use striped_map::StripedHashMap;

use std::hash::Hash;
use std::ops::Deref;

pub trait Map<'a, K: Hash + PartialEq, V, VRef: 'a + Deref> {
    fn get(&'a self, key: &K) -> Option<VRef>;
    fn contains(&self, key: &K) -> bool;
    fn put(&self, key: K, value: V);
    fn remove(&self, key: &K) -> bool;
}
