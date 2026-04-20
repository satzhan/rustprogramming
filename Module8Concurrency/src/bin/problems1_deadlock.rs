use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

fn main() {
    let resource_a = Arc::new(Mutex::new("Data A"));
    let resource_b = Arc::new(Mutex::new("Data B"));

    let a1 = Arc::clone(&resource_a);
    let b1 = Arc::clone(&resource_b);

    let handle1 = thread::spawn(move || {
        // Step 1: Thread 1 locks Resource A
        let _lock_a = a1.lock().unwrap();
        println!("Thread 1: Locked Resource A");

        thread::sleep(Duration::from_millis(50));

        println!("Thread 1: Trying to lock Resource B...");
        let _lock_b = b1.lock().unwrap(); 
        
        println!("Thread 1: Got both locks!"); // This will never print
    });

    let a2 = Arc::clone(&resource_a);
    let b2 = Arc::clone(&resource_b);

    let handle2 = thread::spawn(move || {
        let _lock_b = b2.lock().unwrap();
        println!("Thread 2: Locked Resource B");

        thread::sleep(Duration::from_millis(50));

        println!("Thread 2: Trying to lock Resource A...");
        let _lock_a = a2.lock().unwrap();
        
        println!("Thread 2: Got both locks!"); // This will never print
    });

    handle1.join().unwrap();
    handle2.join().unwrap();

    println!("Program finished successfully!"); // This will never print either!
}