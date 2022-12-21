use std::hash::{BuildHasher, Hash};
use std::ops::Deref;
use std::sync::MutexGuard;
use std::{collections::HashMap, sync::Mutex};

use super::Map;

/// A concurrent hashmap implemented with coarse-grained locking.
pub struct CoarseMap<K, V, S>(Mutex<HashMap<K, V, S>>);

pub struct ElemRef<'a, K, V, S> {
    vref: &'a V,
    _guard: MutexGuard<'a, HashMap<K, V, S>>,
}

impl<'a, K, V, S> Deref for ElemRef<'a, K, V, S> {
    type Target = V;

    fn deref(&self) -> &Self::Target {
        self.vref
    }
}

impl<'a, K, V, S> Map<'a, K, V, ElemRef<'a, K, V, S>> for CoarseMap<K, V, S>
where
    K: PartialEq + Eq + Hash + PartialEq,
    S: BuildHasher,
{
    fn get(&self, key: &K) -> Option<ElemRef<'_, K, V, S>> {
        let guard = self.0.lock().unwrap();
        let val = guard.get(key);
        match val {
            Some(vref) => {
                // SAFETY: extending the lifetime of vref is safe here because
                // vref will not be invalidated while the mutex guard is alive.
                // ElemRef ensures the mutex guard and vref will have the same
                // lifetime.
                let vref = unsafe { std::mem::transmute(vref) };
                Some(ElemRef {
                    vref,
                    _guard: guard,
                })
            }
            None => None,
        }
    }

    fn contains(&self, key: &K) -> bool {
        self.0.lock().unwrap().contains_key(key)
    }

    fn put(&self, key: K, value: V) {
        self.0.lock().unwrap().insert(key, value);
    }

    fn remove(&self, key: &K) -> bool {
        self.0.lock().unwrap().remove(key).is_some()
    }
}
