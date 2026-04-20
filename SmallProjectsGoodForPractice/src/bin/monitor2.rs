use std::thread;
use std::time::{Duration, Instant};

fn main() { // simplest example
    let start = Instant::now();
    
    // Spawn the monitor
    let monitor = thread::spawn(move || {
        for _ in 0..5 {
            let elapsed = start.elapsed().as_millis();
            println!("[monitor] tick at {}ms", elapsed);
            thread::sleep(Duration::from_millis(10));
        }
    });
    
    // "Main work" — pretend this is your dispatcher + workers
    thread::sleep(Duration::from_millis(60));
    
    monitor.join().unwrap();
}