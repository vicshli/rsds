//! This module contains concurrent hashmap implementations.

mod coarse_map;
mod striped_map;

pub use coarse_map::CoarseMap;
pub use striped_map::StripedHashMap;

use std::hash::Hash;
use std::ops::Deref;

/// Common functionalities for hash maps.
pub trait Map {
    /// Key type for a HashMap implementation.
    type Key: Hash;
    /// Value type for a HashMap implementation.
    type Val;
    /// HashMap entry reference type.
    type ValueRef<'a>: Deref<Target = Self::Val>
    where
        Self: 'a;

    /// Get reference to a value associated with a key, if it exists.
    fn get(&self, key: &Self::Key) -> Option<Self::ValueRef<'_>>;

    /// Check whether the map contains a value mapped to the given key.
    fn contains(&self, key: &Self::Key) -> bool;

    /// Emplaces a key-value pair into the map.
    ///
    /// If there were a key-value pair associated with this provided key,
    /// it will be overwritten.
    fn put(&self, key: Self::Key, value: Self::Val);

    /// Attempts to remove a key-value pair based on the provided key, returning
    /// whether a key-value pair was found and removed.
    fn remove(&self, key: &Self::Key) -> bool;
}
