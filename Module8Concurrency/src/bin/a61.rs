fn main() {
    use std::sync::{Arc, Mutex};

    let steps = Arc::new(Mutex::new(5));

    let child_steps = Arc::clone(&steps);

    let thread = std::thread::spawn(move || {
        loop {
            std::thread::sleep(std::time::Duration::from_secs(1));

            let mut guard = child_steps.lock().unwrap();
            if *guard <= 0 {
                break;
            }

            println!("Child sees {}", *guard);
            *guard -= 1;
        }
        "Goodbye!"
    });

    for _ in 0..3 {
        std::thread::sleep(std::time::Duration::from_millis(700));
        let guard = steps.lock().unwrap();
        println!("Main sees {}", *guard);
    }

    let result = thread.join().unwrap();
    println!("Thread returned: {}", result);
}