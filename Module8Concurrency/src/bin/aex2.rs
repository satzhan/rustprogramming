use std::sync::{Arc, Mutex};
use std::thread;

fn main() {
    let counter = Arc::new(Mutex::new(0));
    let mut handles = Vec::new();

    for id in 1..=5 {
        let counter = Arc::clone(&counter);

        let handle = thread::spawn(move || {
            for _ in 0..10 {
                let mut num = counter.lock().unwrap();
                *num += 1;
            }
            println!("Thread {id} finished.");
        });

        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }

    let final_value = *counter.lock().unwrap();
    println!("Final counter value: {final_value}");
}