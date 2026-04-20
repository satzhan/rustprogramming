use std::collections::HashMap;

// 1. The Struct: We only use one generic type <T> for the value.
// We hardcode the key to always be a String to keep it simple.
struct Cache<T> {
    storage: HashMap<String, T>,
}

// 2. The Implementation: We implement methods for our Cache<T>
impl<T> Cache<T> {
    
    // Create a brand new, empty cache
    fn new() -> Cache<T> {
        Cache {
            storage: HashMap::new(),
        }
    }

    // Retrieve a value. It returns Option<&T> because the key might not exist!
    fn get(&self, key: &str) -> Option<&T> {
        self.storage.get(key)
    }

    // Insert a new key-value pair
    fn set(&mut self, key: String, value: T) {
        self.storage.insert(key, value);
    }
}

fn main() {
    // Let's create a cache where the generic <T> is an i32 (integer)
    let mut score_cache = Cache::new();
    
    // We insert data using String keys and i32 values
    score_cache.set(String::from("Alice"), 100);
    score_cache.set(String::from("Bob"), 85);

    // Let's try to get Alice's score
    match score_cache.get("Alice") {
        Some(score) => println!("Alice's score is: {}", score),
        None => println!("Alice not found in cache."),
    }

    // Let's try to get a score for someone not in the cache
    match score_cache.get("Charlie") {
        Some(score) => println!("Charlie's score is: {}", score),
        None => println!("Charlie not found in cache."),
    }
    let mut explicit_cache = Cache::<String>::new(); 
    explicit_cache.set(String::from("Bob"), String::from("Winner"));
}