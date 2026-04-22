use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

// What the monitor reports back to main at the end of the run
struct MonitorReport {
    sample_count: u64,
    cpu_sum: u64,
    active_sum: u64,
}

fn main() { // cpu usage and activity
    let start = Instant::now();
    let completed = Arc::new(AtomicUsize::new(0));
    let cpu_usage = Arc::new(AtomicUsize::new(0));
    let active_workers = Arc::new(AtomicUsize::new(0));
    let shutdown = Arc::new(AtomicBool::new(false));
    
    // Channel for the monitor to hand back its final report
    let (report_tx, report_rx) = mpsc::channel::<MonitorReport>();
    
    // Spawn 4 workers (unchanged)
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
    
    // Monitor — now accumulates AND reports at end
    let monitor_counter = Arc::clone(&completed);
    let monitor_cpu = Arc::clone(&cpu_usage);
    let monitor_active = Arc::clone(&active_workers);
    let monitor_shutdown = Arc::clone(&shutdown);
    let monitor = thread::spawn(move || {
        // Local state — private to this thread, no sharing needed
        let mut sample_count: u64 = 0;
        let mut cpu_sum: u64 = 0;
        let mut active_sum: u64 = 0;
        
        while !monitor_shutdown.load(Ordering::Relaxed) {
            let done = monitor_counter.load(Ordering::Relaxed);
            let cpu = monitor_cpu.load(Ordering::Relaxed);
            let active = monitor_active.load(Ordering::Relaxed);
            
            // Accumulate
            sample_count += 1;
            cpu_sum += cpu as u64;
            active_sum += active as u64;
            
            let elapsed = start.elapsed().as_millis();
            println!(
                "[monitor {:>4}ms] active={}/4  cpu={}%  completed={}",
                elapsed, active, cpu, done
            );
            thread::sleep(Duration::from_millis(10));
        }
        
        // Hand back the summary before exiting
        report_tx.send(MonitorReport {
            sample_count,
            cpu_sum,
            active_sum,
        }).unwrap();
        
        println!("[monitor] shutting down cleanly");
    });
    
    for handle in worker_handles {
        handle.join().unwrap();
    }
    let total_time = start.elapsed();
    shutdown.store(true, Ordering::Relaxed);
    monitor.join().unwrap();
    
    // Collect the monitor's report
    let report = report_rx.recv().unwrap();
    let total_completed = completed.load(Ordering::Relaxed);
    
    // Compute averages as floats — integer division would lose the decimals
    let avg_cpu = report.cpu_sum as f64 / report.sample_count as f64;
    let avg_active = report.active_sum as f64 / report.sample_count as f64;
    
    println!("\n===== FINAL REPORT =====");
    println!("Total time:       {} ms", total_time.as_millis());
    println!("Tasks completed:  {}", total_completed);
    println!("Samples taken:    {}", report.sample_count);
    println!("Average CPU:      {:.1}%", avg_cpu);
    println!("Average active:   {:.2} / 4 workers", avg_active);
    println!("========================");
}