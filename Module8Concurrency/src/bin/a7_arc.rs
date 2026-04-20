fn main() {
    // Arc by itself is good to keep immutable data
    use std::thread;
    use std::sync::Arc;
    
    let some_resource = Arc::new("Hello World".to_string());
    
    let thread_a = {
        let some_resource = some_resource.clone();
        thread::spawn(move || {
            println!("Thread A says: {}", some_resource);
        })
    };
    
    let thread_b = {
        let some_resource = some_resource.clone();
        thread::spawn(move || {
            println!("Thread B says: {}", some_resource);
        })
    };
    
    thread_a.join().unwrap();
    thread_b.join().unwrap();
}