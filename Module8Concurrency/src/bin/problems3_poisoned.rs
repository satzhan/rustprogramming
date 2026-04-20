use std::sync::{Arc, Mutex};
use std::thread;

fn main() {
    let shared_data = Arc::new(Mutex::new(0));

    let data_clone = Arc::clone(&shared_data);

    let handle = thread::spawn(move || {
        let mut data = data_clone.lock().unwrap();
        
        *data += 1;
        println!("Thread: Started updating data to {}", *data);
        // done some work, some process
        // some changes
        panic!("Oh no! The thread crashed while holding the lock!");
        // more code 
        // more data to work on
        // more process to do
    });

    let _ = handle.join();

    println!("Main: Trying to lock the mutex...");
    
    let lock_result = shared_data.lock();

    match lock_result {
        Ok(data) => println!("Main: Got the data! It is: {}", *data),
        Err(poisoned) => {
            println!("Main: YIKES! The mutex is poisoned! {:?}", poisoned);
            
            let recovered_data = poisoned.into_inner();
            println!("Main: Recovered data is: {}", *recovered_data);
        }
    }
}