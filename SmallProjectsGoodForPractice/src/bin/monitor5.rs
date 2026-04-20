use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::thread;
use std::time::{Duration, Instant};

fn main() { // added cpu
    let start = Instant::now();
    let completed = Arc::new(AtomicUsize::new(0));
    let cpu_usage = Arc::new(AtomicUsize::new(0));  // percent, 0..=100
    let shutdown = Arc::new(AtomicBool::new(false));
    
    // Fake worker — "runs" a CPU task (35%) then an IO task (10%), repeats
    let worker_counter = Arc::clone(&completed);
    let worker_cpu = Arc::clone(&cpu_usage);
    let worker = thread::spawn(move || {
        for i in 0..6 {
            let cost = if i % 2 == 0 { 35 } else { 10 };
            worker_cpu.fetch_add(cost, Ordering::Relaxed);      // task starts
            thread::sleep(Duration::from_millis(30));            // task runs
            worker_cpu.fetch_sub(cost, Ordering::Relaxed);      // task ends
            worker_counter.fetch_add(1, Ordering::Relaxed);
        }
    });
    
    // Monitor — now observing two variables
    let monitor_counter = Arc::clone(&completed);
    let monitor_cpu = Arc::clone(&cpu_usage);
    let monitor_shutdown = Arc::clone(&shutdown);
    let monitor = thread::spawn(move || {
        while !monitor_shutdown.load(Ordering::Relaxed) {
            let done = monitor_counter.load(Ordering::Relaxed);
            let cpu = monitor_cpu.load(Ordering::Relaxed);
            let elapsed = start.elapsed().as_millis();
            println!("[monitor {:>4}ms] cpu={}%  completed={}", elapsed, cpu, done);
            thread::sleep(Duration::from_millis(10));
        }
        println!("[monitor] shutting down cleanly");
    });
    
    worker.join().unwrap();
    shutdown.store(true, Ordering::Relaxed);
    monitor.join().unwrap();
}