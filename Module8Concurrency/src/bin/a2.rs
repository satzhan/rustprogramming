use std::thread;
use std::time::Duration;

fn main() {
    let mut handles = vec![];
    for i in 1..=3 {             // copy i
        let handle = thread::spawn(move || {
            println!("Thread {}", i);
        });
        handles.push(handle);
    }
    
    // Wait for the spawned thread to finish before exiting main
    for handle in handles {
        handle.join().unwrap();
    }
    println!("All threads completed.");
}