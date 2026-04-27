use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::thread;
use std::time::{Duration, Instant};

fn main() { // added monitor automatic shutdown
    let start = Instant::now();
    let completed = Arc::new(AtomicUsize::new(0));
    let shutdown = Arc::new(AtomicBool::new(false));
    
    // Fake worker — does a fixed amount of work, then we'll signal shutdown
    let worker_counter = Arc::clone(&completed);
    let worker = thread::spawn(move || {
        for _ in 0..16 {
            thread::sleep(Duration::from_millis(15));
            worker_counter.fetch_add(1, Ordering::Relaxed);
        }
    });
    
    // Monitor — runs until shutdown flag flips
    let monitor_counter = Arc::clone(&completed);
    let monitor_shutdown = Arc::clone(&shutdown);
    let monitor = thread::spawn(move || {
        while !monitor_shutdown.load(Ordering::Relaxed) {
            let n = monitor_counter.load(Ordering::Relaxed);
            let elapsed = start.elapsed().as_millis();
            println!("[monitor {:>4}ms] completed = {}", elapsed, n);
            thread::sleep(Duration::from_millis(10));
        }
        println!("[monitor] shutting down cleanly");
    });
    
    // Wait for worker to finish, THEN signal the monitor
    worker.join().unwrap();
    shutdown.store(true, Ordering::Relaxed);
    
    monitor.join().unwrap();
}