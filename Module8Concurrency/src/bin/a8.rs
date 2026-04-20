fn main() {
    // Mutex guarantees mutual exclusion, so putting your data inside of mutex
    // allows to use mechanisms for lock and unlock, to guarantee that no other threads will be able to mutate your data.
    
    // use std::sync::Mutex;
    // use std::thread;
    
    struct Message(String);
    
    let msg = Message("Hello".to_string());
    
    let mutex = std::sync::Mutex::new(msg);
    let arc = std::sync::Arc::new(mutex);
    let child; // I want to be able to join my thread, later
    
    { 
        let arc = arc.clone();
        child = std::thread::spawn(move || { 
            let mut guard = arc.lock().unwrap();
            guard.0 = "World!".to_string();
            println!("{}", guard.0);
            // unlock will be called at the moment your variable scope ends!!
            // otherwise you gonna hold your mutex locked;
        });
    }
    
    {
        let guard = arc.lock().unwrap();
        println!("{}", guard.0);
    }
    
    child.join().unwrap();
}