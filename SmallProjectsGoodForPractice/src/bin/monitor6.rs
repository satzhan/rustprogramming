use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::thread;
use std::time::{Duration, Instant};

fn main() {
    let start = Instant::now();
    let completed = Arc::new(AtomicUsize::new(0));
    let cpu_usage = Arc::new(AtomicUsize::new(0));
    let shutdown = Arc::new(AtomicBool::new(false));
    
    // Spawn 4 workers
    let mut worker_handles = Vec::new();
    for worker_id in 0..4 {
        let worker_counter = Arc::clone(&completed);
        let worker_cpu = Arc::clone(&cpu_usage);
        let handle = thread::spawn(move || {
            for i in 0..6 {
                let cost = if i % 2 == 0 { 35 } else { 10 };
                worker_cpu.fetch_add(cost, Ordering::Relaxed);
                thread::sleep(Duration::from_millis(30));
                worker_cpu.fetch_sub(cost, Ordering::Relaxed);
                worker_counter.fetch_add(1, Ordering::Relaxed);
                println!("  [worker {}] finished task {}", worker_id, i);
            }
        });
        worker_handles.push(handle);
    }
    
    // Monitor — unchanged
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
    
    // Wait for all workers, THEN shutdown
    for handle in worker_handles {
        handle.join().unwrap();
    }
    shutdown.store(true, Ordering::Relaxed);
    monitor.join().unwrap();
}