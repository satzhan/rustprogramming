use rand::rngs::StdRng;
use rand::{Rng, RngExt, SeedableRng};
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

// ↓↓↓ ALL TUNABLE PARAMETERS LIVE HERE ↓↓↓
#[derive(Clone, Debug)]
struct Config {
    num_workers: usize,
    tasks_per_worker: usize,
    io_probability: f64,          // 0.7 = 70% IO, 30% CPU
    duration_min_ms: u64,
    duration_max_ms: u64,
    cpu_budget: usize,            // max allowed global CPU usage
    monitor_tick_ms: u64,
    rng_base_seed: u64,
}

impl Config {
    // A sensible default — our current "baseline" workload
    fn balanced() -> Self {
        Config {
            num_workers: 5,
            tasks_per_worker: 6,
            io_probability: 0.5,
            duration_min_ms: 20,
            duration_max_ms: 40,
            cpu_budget: 100,
            monitor_tick_ms: 10,
            rng_base_seed: 42,
        }
    }
    
    // A harder workload — what your handout calls "stressed"
    fn stressed() -> Self {
        Config {
            num_workers: 4,
            tasks_per_worker: 10,
            io_probability: 0.3,      // flipped: mostly CPU now
            duration_min_ms: 20,
            duration_max_ms: 60,
            cpu_budget: 100,
            monitor_tick_ms: 10,
            rng_base_seed: 42,
        }
    }
}

struct MonitorReport {
    sample_count: u64,
    cpu_sum: u64,
    active_sum: u64,
}

fn run_simulation(config: &Config) {
    println!("Running with config: {:#?}\n", config);
    
    let start = Instant::now();
    let completed = Arc::new(AtomicUsize::new(0));
    let cpu_usage = Arc::new(AtomicUsize::new(0));
    let active_workers = Arc::new(AtomicUsize::new(0));
    let shutdown = Arc::new(AtomicBool::new(false));
    
    let (report_tx, report_rx) = mpsc::channel::<MonitorReport>();
    
    // Snapshot values we need in worker closures
    let budget = config.cpu_budget;
    let tasks_per_worker = config.tasks_per_worker;
    let io_prob = config.io_probability;
    let dur_min = config.duration_min_ms;
    let dur_max = config.duration_max_ms;
    let base_seed = config.rng_base_seed;
    let num_workers = config.num_workers;
    
    let mut worker_handles = Vec::new();
    for worker_id in 0..num_workers {
        let worker_counter = Arc::clone(&completed);
        let worker_cpu = Arc::clone(&cpu_usage);
        let worker_active = Arc::clone(&active_workers);
        let mut rng = StdRng::seed_from_u64(base_seed + worker_id as u64);
        
        let handle = thread::spawn(move || {
            for i in 0..tasks_per_worker {
                let kind = if rng.random_bool(io_prob) { TaskKind::Io } else { TaskKind::Cpu };
                let duration_ms = rng.random_range(dur_min..=dur_max);
                let cost = kind.cpu_cost();
                
                // Admission gate
                loop {
                    let result = worker_cpu.fetch_update(
                        Ordering::Relaxed,
                        Ordering::Relaxed,
                        |current| {
                            if current + cost <= budget { Some(current + cost) } else { None }
                        },
                    );
                    if result.is_ok() { break; }
                    thread::sleep(Duration::from_millis(5));
                }
                
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
    
    // Monitor
    let monitor_counter = Arc::clone(&completed);
    let monitor_cpu = Arc::clone(&cpu_usage);
    let monitor_active = Arc::clone(&active_workers);
    let monitor_shutdown = Arc::clone(&shutdown);
    let tick_ms = config.monitor_tick_ms;
    let display_workers = config.num_workers;
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
            println!(
                "[monitor {:>4}ms] active={}/{}  cpu={}%  completed={}",
                elapsed, active, display_workers, cpu, done
            );
            thread::sleep(Duration::from_millis(tick_ms));
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
    println!("Average active:   {:.2} / {} workers", avg_active, display_workers);
    println!("========================\n");
}

fn main() {
    let config = Config::balanced();
    run_simulation(&config);
    
    // Try this: uncomment to run both back-to-back
    println!("\n\n========== STRESSED WORKLOAD ==========\n");
    run_simulation(&Config::stressed());
}