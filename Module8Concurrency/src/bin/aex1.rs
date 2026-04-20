use std::thread;

fn main() {
    let mut handles = Vec::new();

    for id in 1..=3 {
        let handle = thread::spawn(move || {
            println!("Thread {id}");
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }

    println!("All threads completed.");
}