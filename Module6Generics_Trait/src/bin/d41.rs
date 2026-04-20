// Run this to see a fully functional, type-safe data structure.
use std::collections::HashMap;
use std::hash::Hash;
use std::time::{Duration, Instant};

// K must implement Eq and Hash to be used as a HashMap key.
struct ExpiringCache<K, V> where K: Eq + Hash {
    storage: HashMap<K, (V, Instant)>,
    default_ttl: Duration,
}

impl<K, V> ExpiringCache<K, V> where K: Eq + Hash {
    fn new(default_ttl: Duration) -> Self {
        ExpiringCache {
            storage: HashMap::new(),
            default_ttl,
        }
    }

    fn insert(&mut self, key: K, value: V, ttl: Option<Duration>) {
        let expiration = Instant::now() + ttl.unwrap_or(self.default_ttl);
        self.storage.insert(key, (value, expiration));
    }

    fn get(&mut self, key: &K) -> Option<&V> {
        let expired = if let Some((_, expiration)) = self.storage.get(key) {
            Instant::now() > *expiration
        } else {
            false
        };
    
        if expired {
            self.storage.remove(key);
            None
        } else {
            self.storage.get(key).map(|(value, _)| value)
        }
    }
}

fn main() {
    let mut cache = ExpiringCache::new(Duration::new(2, 0)); 
    cache.insert("user_id_1", "Alice", None);

    match cache.get(&"user_id_1") {
        Some(value) => println!("Retrieved immediately: {}", value),
        None => println!("Not found or expired"),
    }

    println!("Waiting for 3 seconds...");
    std::thread::sleep(Duration::new(3, 0));

    match cache.get(&"user_id_1") {
        Some(value) => println!("Retrieved later: {}", value),
        None => println!("Not found or expired (as expected!)"),
    }
}