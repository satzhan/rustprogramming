fn main() {
    use std::sync::Arc; // atomic reference counter (smart pointer)
    use std::sync::Mutex; // mutex -> mutual exclusive

    let total = Arc::new(Mutex::new(0));
    let mut handles = vec![];
    for _ in 0..5 {
        let cnt = total.clone();
        let handle = std::thread::spawn(move || {
            for _ in 0..10 {
                *cnt.lock().unwrap() += 1;
            }
        });
        handles.push(handle);
    };

    for handle in handles {
        handle.join().unwrap();
    }
    println!("Result: {}", *total.lock().unwrap());
}