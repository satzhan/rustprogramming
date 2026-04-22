use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::thread;
use std::time::{Duration, Instant};

fn main() { // added total time
    let start = Instant::now();  // <-- the anchor point
    let completed = Arc::new(AtomicUsize::new(0));
    let cpu_usage = Arc::new(AtomicUsize::new(0));
    let active_workers = Arc::new(AtomicUsize::new(0));
    let shutdown = Arc::new(AtomicBool::new(false));
    
    let mut worker_handles = Vec::new();
    for worker_id in 0..4 {
        let worker_counter = Arc::clone(&completed);
        let worker_cpu = Arc::clone(&cpu_usage);
        let worker_active = Arc::clone(&active_workers);
        let handle = thread::spawn(move || {
            for i in 0..6 {
                let cost = if i % 2 == 0 { 35 } else { 10 };
                worker_active.fetch_add(1, Ordering::Relaxed);
                worker_cpu.fetch_add(cost, Ordering::Relaxed);
                thread::sleep(Duration::from_millis(30));
                worker_cpu.fetch_sub(cost, Ordering::Relaxed);
                worker_active.fetch_sub(1, Ordering::Relaxed);
                worker_counter.fetch_add(1, Ordering::Relaxed);
                println!("  [worker {}] finished task {}", worker_id, i);
            }
        });
        worker_handles.push(handle);
    }
    
    let monitor_counter = Arc::clone(&completed);
    let monitor_cpu = Arc::clone(&cpu_usage);
    let monitor_active = Arc::clone(&active_workers);
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
    
    // Wait for all workers
    for handle in worker_handles {
        handle.join().unwrap();
    }
    
    // ↓↓↓ THE NEW PART ↓↓↓
    // All work is done — capture the total time BEFORE we spin down the monitor.
    let total_time = start.elapsed();
    
    shutdown.store(true, Ordering::Relaxed);
    monitor.join().unwrap();
    
    // Final report
    let total_completed = completed.load(Ordering::Relaxed);
    println!("\n===== FINAL REPORT =====");
    println!("Total time:      {} ms", total_time.as_millis());
    println!("Tasks completed: {}", total_completed);
    println!("========================");
}