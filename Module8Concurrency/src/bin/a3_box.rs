fn main() {
    println!("Intro to Concurrency");
    let steps = Box::new(5);
    let cnt = Box::new(5);
    let thread = std::thread::spawn(move || {
        // important to notice usage of closure
        // it captures the environment, and steps
        // variable becomes available in our new thread
        for step in 1..=*steps {
            std::thread::sleep(std::time::Duration::from_secs(1));
            println!("Thread step {}", step);
        }
        "Goodbye!" // important thread could return values
    });

    println!("Spawned a thread!");

    // Very important moment to understand closure captures
    // the environment
    
    println!("steps now unavailable {}", steps);
    std::thread::sleep(std::time::Duration::from_secs(3));
    println!("Main thread slept for 3 seconds");
    // Now we join our spawned thread with its returned value. If we don't, our function just returns
    // without waiting for the spawned thread.
    let result = thread.join().unwrap(); // we need to unwrap the result enum, because potentially the thread could panic and we end up with an err

    println!("Thread returned: {:?}", result);
}