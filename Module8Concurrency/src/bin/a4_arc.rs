fn main() {
    use std::sync::Arc; // atomic reference counter (smart pointer)
    
    println!("Intro to Concurrency");
    let steps = Arc::new(5);
    let thread = {
        let steps2 = steps.clone();
        std::thread::spawn(move || {
            for step in 1..=*steps2 {
                std::thread::sleep(std::time::Duration::from_secs(1));
                println!("Thread step {}", step);
            }
            "Goodbye!" // important thread could return values
        })
    };

    println!("Spawned a thread!");

    // Very important moment to understand closure captures
    // the environment
    
    println!("steps now available {}", steps);
    std::thread::sleep(std::time::Duration::from_secs(3));
    println!("steps now available {}", steps);
    println!("Main thread slept for 3 seconds");
    // Now we join our spawned thread with its returned value. If we don't, our function just returns
    // without waiting for the spawned thread.
    let result = thread.join().unwrap(); // we need to unwrap the result enum, because potentially the thread could panic and we end up with an err

    println!("Thread returned: {:?}", result);
}
