use std::hash::Hash;

use dashmap::DashMap;
use rand::{distributions::Alphanumeric, Rng};
use rsds::{Map, StripedHashMap};
use std::time::Instant;

fn make_random_string() -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(7)
        .map(char::from)
        .collect()
}

fn make_random_string_pairs(n: usize) -> Vec<(String, String)> {
    (0..n)
        .map(|_| (make_random_string(), make_random_string()))
        .collect()
}

fn bench_single_threaded<K: Hash + PartialEq + Eq + Clone, V: Clone>(src: Vec<(K, V)>) {
    let dmap_data = src.clone();

    let now = Instant::now();
    let map = StripedHashMap::new();
    for (key, val) in src {
        map.put(key, val);
    }

    let elapsed = now.elapsed();
    println!("StripedHashMap Elapsed: {:.2?}", elapsed);

    let now = Instant::now();
    let dmap = DashMap::new();
    for (key, val) in dmap_data {
        dmap.insert(key, val);
    }
    let elapsed = now.elapsed();
    println!("DashMap Elapsed: {:.2?}", elapsed);
}

fn main() {
    let input = make_random_string_pairs(1_000_000);
    bench_single_threaded(input);
}
