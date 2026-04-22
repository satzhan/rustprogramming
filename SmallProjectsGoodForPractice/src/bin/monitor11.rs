use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use rand::RngExt;

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug)]
enum TaskKind {
    Cpu,
    Io,
}

impl TaskKind {
    fn cpu_cost(&self) -> usize {
        match self {
            TaskKind::Cpu => 35,
            TaskKind::Io => 10,
        }
    }
}

const CPU_BUDGET: usize = 100;  // the rule: total CPU can't exceed this

struct MonitorReport {
    sample_count: u64,
    cpu_sum: u64,
    active_sum: u64,
}

fn main() { // added condition to not exceed CPU limit
    let start = Instant::now();
    let completed = Arc::new(AtomicUsize::new(0));
    let cpu_usage = Arc::new(AtomicUsize::new(0));
    let active_workers = Arc::new(AtomicUsize::new(0));
    let shutdown = Arc::new(AtomicBool::new(false));
    
    let (report_tx, report_rx) = mpsc::channel::<MonitorReport>();
    
    let mut worker_handles = Vec::new();
    for worker_id in 0..4 {
        let worker_counter = Arc::clone(&completed);
        let worker_cpu = Arc::clone(&cpu_usage);
        let worker_active = Arc::clone(&active_workers);
        let mut rng = StdRng::seed_from_u64(42 + worker_id as u64);
        
        let handle = thread::spawn(move || {
            for i in 0..6 {
                let kind = if rng.random_bool(0.5) { TaskKind::Io } else { TaskKind::Cpu };
                let duration_ms = rng.random_range(20..=40);
                let cost = kind.cpu_cost();
                
                // ↓↓↓ THE GATE ↓↓↓
                // Try to reserve CPU budget. Spin-retry if the system is full.
                loop {
                    let result = worker_cpu.fetch_update(
                        Ordering::Relaxed,
                        Ordering::Relaxed,
                        |current| {
                            if current + cost <= CPU_BUDGET {
                                Some(current + cost)    // reservation succeeded
                            } else {
                                None                    // no room, refuse
                            }
                        },
                    );
                    if result.is_ok() {
                        break;  // we got our budget
                    }
                    // Couldn't reserve — back off briefly and try again
                    thread::sleep(Duration::from_millis(5));
                }
                
                // Past the gate. Now run the task.
                worker_active.fetch_add(1, Ordering::Relaxed);
                thread::sleep(Duration::from_millis(duration_ms));
                worker_cpu.fetch_sub(cost, Ordering::Relaxed);
                worker_active.fetch_sub(1, Ordering::Relaxed);
                worker_counter.fetch_add(1, Ordering::Relaxed);
                
                println!(
                    "  [worker {}] finished task {} ({:?}, {}ms)",
                    worker_id, i, kind, duration_ms
                );
            }
        });
        worker_handles.push(handle);
    }
    
    // Monitor — unchanged
    let monitor_counter = Arc::clone(&completed);
    let monitor_cpu = Arc::clone(&cpu_usage);
    let monitor_active = Arc::clone(&active_workers);
    let monitor_shutdown = Arc::clone(&shutdown);
    let monitor = thread::spawn(move || {
        let mut sample_count: u64 = 0;
        let mut cpu_sum: u64 = 0;
        let mut active_sum: u64 = 0;
        
        while !monitor_shutdown.load(Ordering::Relaxed) {
            let done = monitor_counter.load(Ordering::Relaxed);
            let cpu = monitor_cpu.load(Ordering::Relaxed);
            let active = monitor_active.load(Ordering::Relaxed);
            
            sample_count += 1;
            cpu_sum += cpu as u64;
            active_sum += active as u64;
            
            let elapsed = start.elapsed().as_millis();
            println!("[monitor {:>4}ms] active={}/4  cpu={}%  completed={}", elapsed, active, cpu, done);
            thread::sleep(Duration::from_millis(10));
        }
        
        report_tx.send(MonitorReport { sample_count, cpu_sum, active_sum }).unwrap();
    });
    
    for handle in worker_handles { handle.join().unwrap(); }
    let total_time = start.elapsed();
    shutdown.store(true, Ordering::Relaxed);
    monitor.join().unwrap();
    
    let report = report_rx.recv().unwrap();
    let total_completed = completed.load(Ordering::Relaxed);
    let avg_cpu = report.cpu_sum as f64 / report.sample_count as f64;
    let avg_active = report.active_sum as f64 / report.sample_count as f64;
    
    println!("\n===== FINAL REPORT =====");
    println!("Total time:       {} ms", total_time.as_millis());
    println!("Tasks completed:  {}", total_completed);
    println!("Average CPU:      {:.1}%", avg_cpu);
    println!("Average active:   {:.2} / 4 workers", avg_active);
    println!("========================");
}