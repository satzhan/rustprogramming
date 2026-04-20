use std::sync::{mpsc, Arc, Mutex, Condvar};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant};

struct SimpleRng {
    state: u64,
}

impl SimpleRng {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn gen_bool(&mut self, probability: f64) -> bool {
        self.state = self.state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let random_u32 = (self.state >> 32) as u32;
        (random_u32 as f64 / u32::MAX as f64) < probability
    }
}

#[derive(Clone, Copy)]
enum TaskKind {
    Io,
    Cpu,
}

struct Task {
    id: usize,
    kind: TaskKind,
    arrival_time: Instant,
}

struct TaskResult {
    kind: TaskKind,
    wait_time: Duration,
    turnaround_time: Duration,
}

fn main() {
    let (task_tx, task_rx) = mpsc::channel::<Task>();
    let shared_task_rx = Arc::new(Mutex::new(task_rx));
    
    let (result_tx, result_rx) = mpsc::channel::<TaskResult>();
    let budget = Arc::new((Mutex::new(0), Condvar::new()));

    let sim_running = Arc::new(AtomicBool::new(true));
    let cpu_history = Arc::new(Mutex::new(Vec::new()));

    let monitor_running = Arc::clone(&sim_running);
    let monitor_budget = Arc::clone(&budget);
    let monitor_history = Arc::clone(&cpu_history);

    let monitor_handle = thread::spawn(move || {
        let mut time_ms = 0;
        while monitor_running.load(Ordering::Relaxed) {
            let current_usage = {
                let lock = monitor_budget.0.lock().unwrap();
                *lock
            };
            
            monitor_history.lock().unwrap().push((time_ms, current_usage));
            thread::sleep(Duration::from_millis(10));
            time_ms += 10;
        }
    });

    let mut workers = Vec::new();

    for _ in 0..10 {
        let rx = Arc::clone(&shared_task_rx);
        let budget = Arc::clone(&budget);
        let res_tx = result_tx.clone();

        workers.push(thread::spawn(move || {
            loop {
                let task_result = {
                    let lock = rx.lock().unwrap();
                    lock.recv()
                };

                let task = match task_result {
                    Ok(t) => t,
                    Err(_) => break,
                };

                let start_time = Instant::now();
                let wait_time = start_time.duration_since(task.arrival_time);
                let (lock, cvar) = &*budget;

                match task.kind {
                    TaskKind::Io => {
                        let mut used = lock.lock().unwrap();
                        while *used > 90 {
                            used = cvar.wait(used).unwrap();
                        }
                        *used += 10;
                        drop(used);

                        thread::sleep(Duration::from_millis(200));

                        let mut used = lock.lock().unwrap();
                        *used -= 10;
                        cvar.notify_all();
                    }
                    TaskKind::Cpu => {
                        let mut used = lock.lock().unwrap();
                        while *used > 50 {
                            used = cvar.wait(used).unwrap();
                        }
                        *used += 50;
                        drop(used);

                        thread::sleep(Duration::from_millis(200));

                        let mut mutable_used = lock.lock().unwrap();
                        *mutable_used -= 50;
                        cvar.notify_all();
                    }
                }

                let finish_time = Instant::now();
                let turnaround_time = finish_time.duration_since(task.arrival_time);

                res_tx.send(TaskResult {
                    kind: task.kind,
                    wait_time,
                    turnaround_time,
                }).unwrap();
            }
        }));
    }

    let simulation_start = Instant::now();
    let mut rng = SimpleRng::new(42);
    let total_tasks = 1000;

    for id in 0..total_tasks {
        let kind = if rng.gen_bool(0.65) { TaskKind::Io } else { TaskKind::Cpu };
        let task = Task {
            id,
            kind,
            arrival_time: Instant::now(),
        };
        
        task_tx.send(task).unwrap();
        thread::sleep(Duration::from_millis(20));
    }

    drop(task_tx);
    drop(result_tx);

    let mut total_wait_time = Duration::new(0, 0);
    let mut total_turnaround_time = Duration::new(0, 0);
    let mut io_count = 0;
    let mut cpu_count = 0;

    for res in result_rx {
        total_wait_time += res.wait_time;
        total_turnaround_time += res.turnaround_time;
        match res.kind {
            TaskKind::Io => io_count += 1,
            TaskKind::Cpu => cpu_count += 1,
        }
    }

    for worker in workers {
        worker.join().unwrap();
    }

    sim_running.store(false, Ordering::Relaxed);
    monitor_handle.join().unwrap();

    let simulation_duration = simulation_start.elapsed();
    let history = cpu_history.lock().unwrap();
    
    let total_samples = history.len();
    let mut sum_cpu = 0;
    let mut max_cpu = 0;
    
    for &(_, usage) in history.iter() {
        sum_cpu += usage;
        if usage > max_cpu {
            max_cpu = usage;
        }
    }
    
    let average_cpu = if total_samples > 0 {
        sum_cpu as f64 / total_samples as f64
    } else {
        0.0
    };

    println!("--- SCHEDULER STATISTICS ---");
    println!("Total Tasks Processed: {} (IO: {}, CPU: {})", total_tasks, io_count, cpu_count);
    println!("Total Simulation Time: {:?}", simulation_duration);
    println!("Average Wait Time: {:?}", total_wait_time / total_tasks as u32);
    println!("Average Turnaround Time: {:?}", total_turnaround_time / total_tasks as u32);
    println!("Average CPU Usage: {:.2} units", average_cpu);
    println!("Peak CPU Usage: {} units", max_cpu);
    println!("----------------------------");
}