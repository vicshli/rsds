use std::hash::Hash;

use dashmap::DashMap;
use rand::{distributions::Alphanumeric, Rng};
use rsds::{Map, StripedHashMap};
use std::sync::Arc;
use std::sync::Barrier;
use std::thread;
use std::time::Instant;

const NUM_BUCKETS: usize = 100;

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

fn partition_data<T>(data: Vec<T>, num_partitions: usize) -> Vec<Vec<T>> {
    match num_partitions {
        0 => unimplemented!(),
        1 => vec![data],
        n => {
            let partition_sz = data.len() / n;
            let mut out = Vec::new();
            let mut c = 0;
            let mut buf = Vec::new();
            for item in data {
                buf.push(item);
                c += 1;
                if c == partition_sz {
                    c = 0;
                    out.push(buf);
                    buf = Vec::new();
                }
            }
            if !buf.is_empty() {
                out.push(buf);
            }
            out
        }
    }
}

fn bench_single_threaded<K: Hash + PartialEq + Eq + Clone, V: Clone>(src: &Vec<(K, V)>) {
    println!("bench single threaded");

    let map_data = src.clone();
    bench!("StripedHashMap", {
        let map = StripedHashMap::with_num_buckets(NUM_BUCKETS);
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

fn bench_multi_threaded<
    K: Hash + PartialEq + Eq + Clone + Send + Sync + 'static,
    V: Clone + Send + Sync + 'static,
>(
    num_threads: usize,
    src: &Vec<(K, V)>,
) {
    println!("bench multi threaded");

    let map_data = src.clone();
    let thread_data = partition_data(map_data, num_threads);
    let map = Arc::new(StripedHashMap::with_num_buckets(NUM_BUCKETS));
    let start_barr = Arc::new(Barrier::new(num_threads + 1));
    let end_barr = Arc::new(Barrier::new(num_threads + 1));

    let mut handles = Vec::new();
    for data in thread_data {
        let tmap = map.clone();
        let t_start_barr = start_barr.clone();
        let t_end_barr = end_barr.clone();
        handles.push(thread::spawn(move || {
            t_start_barr.wait();
            for (key, val) in data {
                tmap.put(key, val);
            }
            t_end_barr.wait();
        }));
    }

    handles.push(thread::spawn(move || {
        start_barr.wait();
        let now = Instant::now();
        end_barr.wait();
        let elapsed = now.elapsed();
        println!("StripedHashMap multithreaded elapsed: {:.2?}", elapsed);
    }));

    for h in handles {
        h.join().unwrap();
    }

    let dmap_data = src.clone();
    let thread_data = partition_data(dmap_data, num_threads);
    let dmap = Arc::new(DashMap::new());
    let start_barr = Arc::new(Barrier::new(num_threads + 1));
    let end_barr = Arc::new(Barrier::new(num_threads + 1));

    let mut handles = Vec::new();
    for data in thread_data {
        let tmap = dmap.clone();
        let t_start_barr = start_barr.clone();
        let t_end_barr = end_barr.clone();
        handles.push(thread::spawn(move || {
            t_start_barr.wait();
            for (key, val) in data {
                tmap.insert(key, val);
            }
            t_end_barr.wait();
        }));
    }

    handles.push(thread::spawn(move || {
        start_barr.wait();
        let now = Instant::now();
        end_barr.wait();
        let elapsed = now.elapsed();
        println!("DashMap multithreaded elapsed: {:.2?}", elapsed);
    }));

    for h in handles {
        h.join().unwrap();
    }
}

fn main() {
    let input = make_random_string_pairs(10_000_000);
    bench_single_threaded(&input);
    bench_multi_threaded(10, &input);
}
