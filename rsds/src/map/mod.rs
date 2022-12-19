//! This module contains concurrent hashmap implementations.

mod striped_map;

pub use striped_map::StripedHashMap;

use std::hash::Hash;
use std::ops::Deref;

/// Common functionalities for hash maps.
pub trait Map<'a, K: Hash + PartialEq, V, VRef: 'a + Deref> {
    /// Get reference to a value associated with a key, if it exists.
    fn get(&'a self, key: &K) -> Option<VRef>;

    /// Check whether the map contains a value mapped to the given key.
    fn contains(&self, key: &K) -> bool;

    /// Emplaces a key-value pair into the map.
    ///
    /// If there were a key-value pair associated with this provided key,
    /// it will be overwritten.
    fn put(&self, key: K, value: V);

    /// Attempts to remove a key-value pair based on the provided key, returning
    /// whether a key-value pair was found and removed.
    fn remove(&self, key: &K) -> bool;
}
