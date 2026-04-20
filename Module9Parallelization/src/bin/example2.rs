use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

// --------------------
// Tunable parameters
// --------------------
const NUM_WORKERS: usize = 10;
const TOTAL_TASKS: usize = 1000;

const CPU_BUDGET: u32 = 100;
const CPU_TASK_COST: u32 = 50;

const CPU_TASK_DURATION_MS: u64 = 200;
const IO_TASK_DURATION_MS: u64 = 200;

const IO_PROBABILITY: f64 = 0.65;
const ARRIVAL_GAP_MS: u64 = 20;

// Keep a couple of workers effectively "available" for CPU-heavy work
// when both queues are busy.
const RESERVE_WORKERS_FOR_CPU: usize = 2;

// --------------------
// Small RNG, same spirit as your version
// --------------------
struct SimpleRng {
    state: u64,
}

impl SimpleRng {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn gen_bool(&mut self, probability: f64) -> bool {
        self.state = self
            .state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);

        let random_u32 = (self.state >> 32) as u32;
        (random_u32 as f64 / u32::MAX as f64) < probability
    }
}

// --------------------
// Core task types
// --------------------
#[derive(Clone, Copy, Debug)]
enum TaskKind {
    Io,
    Cpu,
}

#[derive(Clone, Debug)]
struct Task {
    id: usize,
    kind: TaskKind,
    arrival_time: Instant,
}

#[derive(Debug)]
struct TaskResult {
    kind: TaskKind,
    wait_time: Duration,
    turnaround_time: Duration,
}

// --------------------
// Messages
// --------------------

// What scheduler sends to a worker
enum WorkerCommand {
    Run(Task),
    Shutdown,
}

// Single scheduler inbox:
// both producer and workers talk to scheduler through this.
enum SchedulerEvent {
    NewTask(Task),
    WorkerFinished { worker_id: usize, task: Task },
    ProducerDone,
}

// --------------------
// Scheduler state
// --------------------
struct SchedulerState {
    cpu_queue: VecDeque<Task>,
    io_queue: VecDeque<Task>,
    free_workers: Vec<usize>,
    cpu_in_use: u32,
    running_tasks: usize,
    producer_done: bool,
}

impl SchedulerState {
    fn new(num_workers: usize) -> Self {
        Self {
            cpu_queue: VecDeque::new(),
            io_queue: VecDeque::new(),
            free_workers: (0..num_workers).collect(),
            cpu_in_use: 0,
            running_tasks: 0,
            producer_done: false,
        }
    }

    fn can_run_cpu(&self) -> bool {
        self.cpu_in_use + CPU_TASK_COST <= CPU_BUDGET
    }

    fn oldest_cpu_wait(&self) -> Duration {
        self.cpu_queue
            .front()
            .map(|t| t.arrival_time.elapsed())
            .unwrap_or(Duration::ZERO)
    }

    fn oldest_io_wait(&self) -> Duration {
        self.io_queue
            .front()
            .map(|t| t.arrival_time.elapsed())
            .unwrap_or(Duration::ZERO)
    }

    fn is_drained(&self) -> bool {
        self.producer_done
            && self.cpu_queue.is_empty()
            && self.io_queue.is_empty()
            && self.running_tasks == 0
    }
}

// --------------------
// Scheduling policy
// --------------------
//
// Very simple policy:
//
// 1. Keep two separate waiting queues: CPU and IO.
// 2. Prefer CPU if:
//    - CPU fits current budget, and
//    - CPU has waited at least as long as IO,
//      OR we currently have plenty of free workers.
// 3. Otherwise run IO.
// 4. If only one side has tasks, use that side.
// 5. If CPU cannot fit the budget right now, do not dispatch it yet.
//
fn pick_next_task(state: &mut SchedulerState) -> Option<Task> {
    let has_cpu = !state.cpu_queue.is_empty();
    let has_io = !state.io_queue.is_empty();

    match (has_cpu, has_io) {
        (false, false) => None,

        (true, false) => {
            if state.can_run_cpu() {
                state.cpu_queue.pop_front()
            } else {
                None
            }
        }

        (false, true) => state.io_queue.pop_front(),

        (true, true) => {
            let cpu_wait = state.oldest_cpu_wait();
            let io_wait = state.oldest_io_wait();

            if state.can_run_cpu()
                && (cpu_wait >= io_wait || state.free_workers.len() > RESERVE_WORKERS_FOR_CPU)
            {
                state.cpu_queue.pop_front()
            } else {
                state.io_queue.pop_front()
            }
        }
    }
}

fn dispatch_ready_tasks(
    state: &mut SchedulerState,
    worker_senders: &[mpsc::Sender<WorkerCommand>],
    shared_cpu_usage: &Arc<Mutex<u32>>,
) {
    while !state.free_workers.is_empty() {
        let Some(task) = pick_next_task(state) else {
            break;
        };

        let worker_id = state.free_workers.pop().unwrap();

        if let TaskKind::Cpu = task.kind {
            state.cpu_in_use += CPU_TASK_COST;
            *shared_cpu_usage.lock().unwrap() = state.cpu_in_use;
        }

        state.running_tasks += 1;

        // If a worker somehow disappeared, put state back as best as we can.
        if worker_senders[worker_id]
            .send(WorkerCommand::Run(task.clone()))
            .is_err()
        {
            state.running_tasks -= 1;
            state.free_workers.push(worker_id);

            if let TaskKind::Cpu = task.kind {
                state.cpu_in_use -= CPU_TASK_COST;
                *shared_cpu_usage.lock().unwrap() = state.cpu_in_use;
                state.cpu_queue.push_front(task);
            } else {
                state.io_queue.push_front(task);
            }

            break;
        }
    }
}

fn main() {
    // Producer/main and workers both send events to scheduler.
    let (scheduler_tx, scheduler_rx) = mpsc::channel::<SchedulerEvent>();

    // Workers send final timing results here for statistics.
    let (result_tx, result_rx) = mpsc::channel::<TaskResult>();

    // Shared CPU usage only for the monitor/statistics thread.
    let sim_running = Arc::new(AtomicBool::new(true));
    let shared_cpu_usage = Arc::new(Mutex::new(0u32));
    let cpu_history = Arc::new(Mutex::new(Vec::new()));

    // --------------------
    // Monitor thread
    // --------------------
    let monitor_running = Arc::clone(&sim_running);
    let monitor_cpu = Arc::clone(&shared_cpu_usage);
    let monitor_history = Arc::clone(&cpu_history);

    let monitor_handle = thread::spawn(move || {
        let mut time_ms = 0;

        while monitor_running.load(Ordering::Relaxed) {
            let current_usage = *monitor_cpu.lock().unwrap();
            monitor_history.lock().unwrap().push((time_ms, current_usage));

            thread::sleep(Duration::from_millis(10));
            time_ms += 10;
        }
    });

    // --------------------
    // Worker pool
    // --------------------
    let mut worker_handles = Vec::new();
    let mut worker_senders = Vec::new();

    for worker_id in 0..NUM_WORKERS {
        let (worker_tx, worker_rx) = mpsc::channel::<WorkerCommand>();
        worker_senders.push(worker_tx);

        let scheduler_tx_clone = scheduler_tx.clone();
        let result_tx_clone = result_tx.clone();

        let handle = thread::spawn(move || {
            loop {
                match worker_rx.recv() {
                    Ok(WorkerCommand::Run(task)) => {
                        let start_time = Instant::now();
                        let wait_time = start_time.duration_since(task.arrival_time);

                        match task.kind {
                            TaskKind::Io => {
                                thread::sleep(Duration::from_millis(IO_TASK_DURATION_MS));
                            }
                            TaskKind::Cpu => {
                                thread::sleep(Duration::from_millis(CPU_TASK_DURATION_MS));
                            }
                        }

                        let finish_time = Instant::now();
                        let turnaround_time = finish_time.duration_since(task.arrival_time);

                        let _ = result_tx_clone.send(TaskResult {
                            kind: task.kind,
                            wait_time,
                            turnaround_time,
                        });

                        let _ = scheduler_tx_clone.send(SchedulerEvent::WorkerFinished {
                            worker_id,
                            task,
                        });
                    }

                    Ok(WorkerCommand::Shutdown) | Err(_) => break,
                }
            }
        });

        worker_handles.push(handle);
    }

    // Main does not need its own result sender anymore.
    drop(result_tx);

    // --------------------
    // Scheduler thread
    // --------------------
    let scheduler_cpu_usage = Arc::clone(&shared_cpu_usage);

    let scheduler_handle = thread::spawn(move || {
        let mut state = SchedulerState::new(NUM_WORKERS);

        while let Ok(event) = scheduler_rx.recv() {
            match event {
                SchedulerEvent::NewTask(task) => match task.kind {
                    TaskKind::Cpu => state.cpu_queue.push_back(task),
                    TaskKind::Io => state.io_queue.push_back(task),
                },

                SchedulerEvent::WorkerFinished { worker_id, task } => {
                    state.free_workers.push(worker_id);
                    state.running_tasks -= 1;

                    if let TaskKind::Cpu = task.kind {
                        state.cpu_in_use -= CPU_TASK_COST;
                        *scheduler_cpu_usage.lock().unwrap() = state.cpu_in_use;
                    }
                }

                SchedulerEvent::ProducerDone => {
                    state.producer_done = true;
                }
            }

            // After every event, try to dispatch as many waiting tasks as possible.
            dispatch_ready_tasks(&mut state, &worker_senders, &scheduler_cpu_usage);

            if state.is_drained() {
                break;
            }
        }

        // Tell all workers to stop.
        for sender in worker_senders {
            let _ = sender.send(WorkerCommand::Shutdown);
        }
    });

    // --------------------
    // Main still acts like your producer:
    // generate 1000 random tasks and send them "somewhere"
    // --------------------
    let simulation_start = Instant::now();
    let mut rng = SimpleRng::new(42);

    for id in 0..TOTAL_TASKS {
        let kind = if rng.gen_bool(IO_PROBABILITY) {
            TaskKind::Io
        } else {
            TaskKind::Cpu
        };

        let task = Task {
            id,
            kind,
            arrival_time: Instant::now(),
        };

        scheduler_tx.send(SchedulerEvent::NewTask(task)).unwrap();
        thread::sleep(Duration::from_millis(ARRIVAL_GAP_MS));
    }

    scheduler_tx.send(SchedulerEvent::ProducerDone).unwrap();
    drop(scheduler_tx);

    // --------------------
    // Gather statistics
    // --------------------
    let mut total_wait_time = Duration::ZERO;
    let mut total_turnaround_time = Duration::ZERO;
    let mut io_count = 0usize;
    let mut cpu_count = 0usize;

    for res in result_rx {
        total_wait_time += res.wait_time;
        total_turnaround_time += res.turnaround_time;

        match res.kind {
            TaskKind::Io => io_count += 1,
            TaskKind::Cpu => cpu_count += 1,
        }
    }

    scheduler_handle.join().unwrap();

    for handle in worker_handles {
        handle.join().unwrap();
    }

    sim_running.store(false, Ordering::Relaxed);
    monitor_handle.join().unwrap();

    let simulation_duration = simulation_start.elapsed();
    let history = cpu_history.lock().unwrap();

    let total_samples = history.len();
    let mut sum_cpu = 0u64;
    let mut max_cpu = 0u32;

    for &(_, usage) in history.iter() {
        sum_cpu += usage as u64;
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
    println!(
        "Total Tasks Processed: {} (IO: {}, CPU: {})",
        TOTAL_TASKS, io_count, cpu_count
    );
    println!("Total Simulation Time: {:?}", simulation_duration);
    println!(
        "Average Wait Time: {:?}",
        total_wait_time / TOTAL_TASKS as u32
    );
    println!(
        "Average Turnaround Time: {:?}",
        total_turnaround_time / TOTAL_TASKS as u32
    );
    println!("Average CPU Usage: {:.2} units", average_cpu);
    println!("Peak CPU Usage: {} units", max_cpu);
    println!("----------------------------");
}