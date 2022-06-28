use std::hash::Hash;

use dashmap::DashMap;
use rand::{distributions::Alphanumeric, Rng};
use rsds::{Map, StripedHashMap};
use std::time::Instant;

macro_rules! bench {
    ($name: expr, $body: expr) => {
        let now = Instant::now();
        $body;
        let elapsed = now.elapsed();
        println!("{} elapsed: {:.2?}", $name, elapsed);
    };
}

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

fn bench_single_threaded<K: Hash + PartialEq + Eq + Clone, V: Clone>(src: &Vec<(K, V)>) {
    let map_data = src.clone();
    bench!("StripedHashMap", {
        let map = StripedHashMap::new();
        for (key, val) in map_data {
            map.put(key, val);
        }
    });

    let dmap_data = src.clone();
    bench!("DashMap", {
        let map = DashMap::new();
        for (key, val) in dmap_data {
            map.insert(key, val);
        }
    });
}

fn main() {
    let input = make_random_string_pairs(1_000_000);
    bench_single_threaded(&input);
}
