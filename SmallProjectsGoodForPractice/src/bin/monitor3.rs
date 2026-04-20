use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;
use std::time::{Duration, Instant};

fn main() {
    let start = Instant::now();
    let completed = Arc::new(AtomicUsize::new(0));
    
    // Fake worker — increments the counter now and then
    let worker_counter = Arc::clone(&completed);
    let worker = thread::spawn(move || {
        for _ in 0..8 {
            thread::sleep(Duration::from_millis(15));
            worker_counter.fetch_add(1, Ordering::Relaxed);
        }
    });
    
    // Monitor — peeks at the counter every 10ms
    let monitor_counter = Arc::clone(&completed);
    let monitor = thread::spawn(move || {
        for _ in 0..15 {
            let n = monitor_counter.load(Ordering::Relaxed);
            let elapsed = start.elapsed().as_millis();
            println!("[monitor {:>4}ms] completed = {}", elapsed, n);
            thread::sleep(Duration::from_millis(10));
        }
    });
    
    worker.join().unwrap();
    monitor.join().unwrap();
}