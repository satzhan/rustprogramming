use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::thread;
use std::time::{Duration, Instant};

fn main() { // added active workers
    let start = Instant::now();
    let completed = Arc::new(AtomicUsize::new(0));
    let cpu_usage = Arc::new(AtomicUsize::new(0));
    let active_workers = Arc::new(AtomicUsize::new(0));  // NEW
    let shutdown = Arc::new(AtomicBool::new(false));
    
    // Spawn 4 workers
    let mut worker_handles = Vec::new();
    for worker_id in 0..4 {
        let worker_counter = Arc::clone(&completed);
        let worker_cpu = Arc::clone(&cpu_usage);
        let worker_active = Arc::clone(&active_workers);  // NEW
        let handle = thread::spawn(move || {
            for i in 0..6 {
                let cost = if i % 2 == 0 { 35 } else { 10 };
                
                // Task starts
                worker_active.fetch_add(1, Ordering::Relaxed);  // NEW
                worker_cpu.fetch_add(cost, Ordering::Relaxed);
                
                thread::sleep(Duration::from_millis(30));
                
                // Task ends
                worker_cpu.fetch_sub(cost, Ordering::Relaxed);
                worker_active.fetch_sub(1, Ordering::Relaxed);  // NEW
                worker_counter.fetch_add(1, Ordering::Relaxed);
                
                println!("  [worker {}] finished task {}", worker_id, i);
            }
        });
        worker_handles.push(handle);
    }
    
    // Monitor — now observes three variables
    let monitor_counter = Arc::clone(&completed);
    let monitor_cpu = Arc::clone(&cpu_usage);
    let monitor_active = Arc::clone(&active_workers);  // NEW
    let monitor_shutdown = Arc::clone(&shutdown);
    let monitor = thread::spawn(move || {
        while !monitor_shutdown.load(Ordering::Relaxed) {
            let done = monitor_counter.load(Ordering::Relaxed);
            let cpu = monitor_cpu.load(Ordering::Relaxed);
            let active = monitor_active.load(Ordering::Relaxed);
            let elapsed = start.elapsed().as_millis();
            println!(
                "[monitor {:>4}ms] active={}/4  cpu={}%  completed={}",
                elapsed, active, cpu, done
            );
            thread::sleep(Duration::from_millis(10));
        }
        println!("[monitor] shutting down cleanly");
    });
    
    for handle in worker_handles {
        handle.join().unwrap();
    }
    shutdown.store(true, Ordering::Relaxed);
    monitor.join().unwrap();
}