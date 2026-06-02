// Concurrent Task Dispatcher — Optimized Scheduler, Fixed Version
//
// This is a self-contained Rust file: it uses only the Rust standard library.
// Put this in src/main.rs and run with:
//
//     cargo run --release
//
// Optional config.toml next to Cargo.toml:
//
//     total_tasks = 1000
//     io_fraction = 0.70
//     arrival_interval_ms = 1
//     num_workers = 8
//     task_duration_ms = 200
//     io_cpu_pct = 10
//     cpu_cpu_pct = 35
//     cpu_cap_pct = 100
//     monitor_interval_ms = 10
//     rng_seed = 42
//     csv_path = "monitor_log_optimized_fixed.csv"
//
// Model:
//   IO task  = sleepy task, low simulated CPU budget usage.
//   CPU task = heavier task, higher simulated CPU budget usage.
//
// Important teaching note:
//   The CPU percentage here is a SIMULATION BUDGET, not actual OS CPU usage.
//   Both IO and CPU tasks use thread::sleep so students can study scheduling policy
//   without burning laptop CPU.
//
// Main fixes compared with the earlier optimized version:
//   1) The drain-IO threshold is actually 6, not 5.
//   2) The policy uses queued + running tasks, not only queued tasks.
//   3) Queue choice and resource booking happen under one mutex, so the manager
//      never pops a task that cannot currently fit.
//   4) Worker release, generator push, and manager dispatch share one Condvar,
//      avoiding missed wakeups between the queue and pool state.
//   5) The generator creates an exact 70/30 mix, then shuffles it deterministically.

use std::collections::VecDeque;
use std::fs;
use std::io::Write;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{mpsc, Arc, Condvar, Mutex};
use std::thread;
use std::time::{Duration, Instant};

// ─────────────────────────── Config ───────────────────────────

#[derive(Debug, Clone)]
struct Config {
    total_tasks: usize,
    io_fraction: f64,
    arrival_interval_ms: u64,
    num_workers: usize,
    task_duration_ms: u64,
    io_cpu_pct: u32,
    cpu_cpu_pct: u32,
    cpu_cap_pct: u32,
    monitor_interval_ms: u64,
    rng_seed: u64,
    csv_path: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            total_tasks: 1000,
            io_fraction: 0.70,
            arrival_interval_ms: 1,
            num_workers: 8,
            task_duration_ms: 200,
            io_cpu_pct: 10,
            cpu_cpu_pct: 35,
            cpu_cap_pct: 100,
            monitor_interval_ms: 10,
            rng_seed: 42,
            csv_path: "monitor_log_optimized_fixed.csv".to_string(),
        }
    }
}

fn load_config() -> Config {
    let mut cfg = Config::default();

    let Ok(text) = fs::read_to_string("config.toml") else {
        return cfg;
    };

    for raw in text.lines() {
        let line = raw.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }

        let Some((key, value)) = line.split_once('=') else {
            continue;
        };

        let key = key.trim();
        let value = value.trim().trim_matches('"');

        match key {
            "total_tasks" => cfg.total_tasks = value.parse().unwrap_or(cfg.total_tasks),
            "io_fraction" => cfg.io_fraction = value.parse().unwrap_or(cfg.io_fraction),
            "arrival_interval_ms" => {
                cfg.arrival_interval_ms = value.parse().unwrap_or(cfg.arrival_interval_ms)
            }
            "num_workers" => cfg.num_workers = value.parse().unwrap_or(cfg.num_workers),
            "task_duration_ms" => cfg.task_duration_ms = value.parse().unwrap_or(cfg.task_duration_ms),
            "io_cpu_pct" => cfg.io_cpu_pct = value.parse().unwrap_or(cfg.io_cpu_pct),
            "cpu_cpu_pct" => cfg.cpu_cpu_pct = value.parse().unwrap_or(cfg.cpu_cpu_pct),
            "cpu_cap_pct" => cfg.cpu_cap_pct = value.parse().unwrap_or(cfg.cpu_cap_pct),
            "monitor_interval_ms" => {
                cfg.monitor_interval_ms = value.parse().unwrap_or(cfg.monitor_interval_ms)
            }
            "rng_seed" => cfg.rng_seed = value.parse().unwrap_or(cfg.rng_seed),
            "csv_path" => cfg.csv_path = value.to_string(),
            _ => {}
        }
    }

    cfg
}

fn validate_config(cfg: &Config) {
    assert!(cfg.total_tasks > 0, "total_tasks must be > 0");
    assert!(
        (0.0..=1.0).contains(&cfg.io_fraction),
        "io_fraction must be between 0.0 and 1.0"
    );
    assert!(cfg.num_workers > 0, "num_workers must be > 0");
    assert!(cfg.cpu_cap_pct > 0, "cpu_cap_pct must be > 0");
    assert!(
        cfg.io_cpu_pct <= cfg.cpu_cap_pct,
        "one IO task must fit inside the CPU budget"
    );
    assert!(
        cfg.cpu_cpu_pct <= cfg.cpu_cap_pct,
        "one CPU task must fit inside the CPU budget"
    );
}

// ─────────────────────────── Simple deterministic RNG ─────────
// SplitMix64-style generator: enough for repeatable classroom simulations.

#[derive(Debug, Clone)]
struct SimpleRng {
    state: u64,
}

impl SimpleRng {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    fn gen_index(&mut self, upper_exclusive: usize) -> usize {
        if upper_exclusive <= 1 {
            0
        } else {
            (self.next_u64() as usize) % upper_exclusive
        }
    }
}

// ─────────────────────────── Task ─────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Io,
    Cpu,
}

#[derive(Debug, Clone, Copy)]
struct Task {
    id: u64,
    kind: Kind,
    cpu_cost: u32,
    duration_ms: u64,
    arrival: Instant,
}

#[derive(Debug, Clone, Copy)]
struct Completion {
    id: u64,
    kind: Kind,
    arrival: Instant,
    start: Instant,
    end: Instant,
}

// ─────────────────────────── Shared scheduler state ───────────

struct Scheduler {
    inner: Mutex<State>,
    cv: Condvar,
    cfg: Config,
}

struct State {
    io: VecDeque<Task>,
    cpu: VecDeque<Task>,
    generator_closed: bool,
    cpu_budget_used: u32,
    busy_workers: usize,
    cpu_running: usize,
}

#[derive(Debug, Clone, Copy)]
struct Snapshot {
    queued_io: usize,
    queued_cpu: usize,
    cpu_budget_used: u32,
    busy_workers: usize,
    cpu_running: usize,
}

#[derive(Debug, Clone, Copy)]
enum Pick {
    Io,
    Cpu,
}

impl Scheduler {
    fn new(cfg: Config) -> Self {
        Self {
            inner: Mutex::new(State {
                io: VecDeque::new(),
                cpu: VecDeque::new(),
                generator_closed: false,
                cpu_budget_used: 0,
                busy_workers: 0,
                cpu_running: 0,
            }),
            cv: Condvar::new(),
            cfg,
        }
    }

    fn push(&self, task: Task) {
        let mut s = self.inner.lock().unwrap();
        match task.kind {
            Kind::Io => s.io.push_back(task),
            Kind::Cpu => s.cpu.push_back(task),
        }
        self.cv.notify_all();
    }

    fn close_generator(&self) {
        let mut s = self.inner.lock().unwrap();
        s.generator_closed = true;
        self.cv.notify_all();
    }

    fn release(&self, task: &Task) {
        let mut s = self.inner.lock().unwrap();
        s.cpu_budget_used = s.cpu_budget_used.saturating_sub(task.cpu_cost);
        s.busy_workers = s.busy_workers.saturating_sub(1);
        if task.kind == Kind::Cpu {
            s.cpu_running = s.cpu_running.saturating_sub(1);
        }
        self.cv.notify_all();
    }

    fn snapshot(&self) -> Snapshot {
        let s = self.inner.lock().unwrap();
        Snapshot {
            queued_io: s.io.len(),
            queued_cpu: s.cpu.len(),
            cpu_budget_used: s.cpu_budget_used,
            busy_workers: s.busy_workers,
            cpu_running: s.cpu_running,
        }
    }

    fn queue_len(&self) -> usize {
        let s = self.inner.lock().unwrap();
        s.io.len() + s.cpu.len()
    }

    // Manager calls this to get exactly one task.
    // The important repair is here: decision + pop + booking are one atomic action
    // under one mutex. We never pop a task unless we can book it immediately.
    fn dispatch_or_done(&self) -> Option<Task> {
        let mut s = self.inner.lock().unwrap();

        loop {
            if s.generator_closed && s.io.is_empty() && s.cpu.is_empty() {
                return None;
            }

            if let Some(pick) = choose_dispatch(&s, &self.cfg) {
                let task = match pick {
                    Pick::Io => s.io.pop_front().expect("policy chose IO but IO queue was empty"),
                    Pick::Cpu => s.cpu.pop_front().expect("policy chose CPU but CPU queue was empty"),
                };

                // Atomic booking: the worker slot and simulated CPU budget are reserved
                // before the task is sent to the worker channel.
                s.cpu_budget_used += task.cpu_cost;
                s.busy_workers += 1;
                if task.kind == Kind::Cpu {
                    s.cpu_running += 1;
                }

                return Some(task);
            }

            // Wait because either queues are empty but generator is still alive,
            // or work exists but no currently admissible task fits the constraints.
            s = self.cv.wait(s).unwrap();
        }
    }
}

// ─────────────────────────── Dispatch policy ──────────────────

fn can_start(s: &State, task: &Task, cfg: &Config) -> bool {
    s.busy_workers < cfg.num_workers && s.cpu_budget_used + task.cpu_cost <= cfg.cpu_cap_pct
}

fn choose_dispatch(s: &State, cfg: &Config) -> Option<Pick> {
    let io_front = s.io.front();
    let cpu_front = s.cpu.front();

    let io_can_start = match io_front {
        Some(t) => can_start(s, t, cfg),
        None => false,
    };
    let cpu_can_start = match cpu_front {
        Some(t) => can_start(s, t, cfg),
        None => false,
    };

    if !io_can_start && !cpu_can_start {
        return None;
    }

    // Running IO = all busy workers that are not currently CPU-kind workers.
    let io_running = s.busy_workers.saturating_sub(s.cpu_running);

    // Fix #2: use queued + running, not only queued.
    let io_total = s.io.len() + io_running;
    let cpu_total = s.cpu.len() + s.cpu_running;

    // No CPU left anywhere in the system: stream IO.
    if cpu_total == 0 {
        return if io_can_start { Some(Pick::Io) } else { None };
    }

    // If no IO is waiting, do useful CPU work up to the physical cap.
    // This prevents the ratio rule from leaving workers idle when there is
    // no queued IO to pair with the next CPU task.
    if s.io.is_empty() {
        return if cpu_can_start { Some(Pick::Cpu) } else { None };
    }

    // Fix #1: the threshold is 6, matching the lane geometry:
    //   6 IO + 1 CPU = 95%
    //   3 IO + 2 CPU = 100%
    let drain_io_mode = io_total >= 6 * cpu_total;
    let cpu_target = if drain_io_mode { 1 } else { 2 };

    if s.cpu_running < cpu_target {
        // We want more CPU on the board, but only if it actually fits now.
        // Fix #3: if CPU does not fit, do not pop it and block; try IO.
        if cpu_can_start {
            Some(Pick::Cpu)
        } else if io_can_start {
            Some(Pick::Io)
        } else {
            None
        }
    } else {
        // CPU concurrency target already reached. Fill remaining lanes with IO.
        if io_can_start {
            Some(Pick::Io)
        } else {
            // Do not violate the CPU target just because no IO fits.
            // Wait for a CPU to finish, which lowers cpu_running.
            None
        }
    }
}

// ─────────────────────────── Generator ────────────────────────

fn make_exact_mix(total: usize, io_fraction: f64, rng: &mut SimpleRng) -> Vec<Kind> {
    let io_count = ((total as f64) * io_fraction).round() as usize;
    let io_count = io_count.min(total);
    let cpu_count = total - io_count;

    let mut kinds = Vec::with_capacity(total);
    kinds.extend(std::iter::repeat(Kind::Io).take(io_count));
    kinds.extend(std::iter::repeat(Kind::Cpu).take(cpu_count));

    // Fisher-Yates shuffle.
    for i in (1..kinds.len()).rev() {
        let j = rng.gen_index(i + 1);
        kinds.swap(i, j);
    }

    kinds
}

fn run_generator(cfg: Config, scheduler: Arc<Scheduler>) {
    let mut rng = SimpleRng::new(cfg.rng_seed);
    let kinds = make_exact_mix(cfg.total_tasks, cfg.io_fraction, &mut rng);
    let interval = Duration::from_millis(cfg.arrival_interval_ms);

    for (id, kind) in kinds.into_iter().enumerate() {
        let cpu_cost = match kind {
            Kind::Io => cfg.io_cpu_pct,
            Kind::Cpu => cfg.cpu_cpu_pct,
        };

        scheduler.push(Task {
            id: id as u64,
            kind,
            cpu_cost,
            duration_ms: cfg.task_duration_ms,
            arrival: Instant::now(),
        });

        if cfg.arrival_interval_ms > 0 {
            thread::sleep(interval);
        }
    }

    scheduler.close_generator();
}

// ─────────────────────────── Manager ──────────────────────────

fn run_manager(scheduler: Arc<Scheduler>, worker_tx: mpsc::Sender<Task>) {
    while let Some(task) = scheduler.dispatch_or_done() {
        worker_tx.send(task).expect("worker channel unexpectedly closed");
    }
    drop(worker_tx);
}

// ─────────────────────────── Worker pool ──────────────────────

type SharedRx = Arc<Mutex<mpsc::Receiver<Task>>>;

fn run_worker(
    _id: usize,
    rx: SharedRx,
    scheduler: Arc<Scheduler>,
    completions: Arc<Mutex<Vec<Completion>>>,
    done_count: Arc<AtomicUsize>,
) {
    loop {
        let task = {
            let guard = rx.lock().unwrap();
            match guard.recv() {
                Ok(task) => task,
                Err(_) => return,
            }
        };

        let start = Instant::now();
        thread::sleep(Duration::from_millis(task.duration_ms));
        let end = Instant::now();

        scheduler.release(&task);

        completions.lock().unwrap().push(Completion {
            id: task.id,
            kind: task.kind,
            arrival: task.arrival,
            start,
            end,
        });

        done_count.fetch_add(1, Ordering::SeqCst);
    }
}

// ─────────────────────────── Monitor ──────────────────────────

#[derive(Debug, Clone, Copy)]
struct MonitorSummary {
    avg_cpu_budget_used: f64,
    avg_workers_active: f64,
    peak_cpu_budget_used: u32,
    peak_workers_active: usize,
    samples: u64,
}

fn run_monitor(
    cfg: Config,
    scheduler: Arc<Scheduler>,
    shutdown: Arc<AtomicBool>,
    t0: Instant,
) -> MonitorSummary {
    let mut file = fs::File::create(&cfg.csv_path).expect("cannot create monitor CSV");
    writeln!(
        file,
        "elapsed_ms,cpu_budget_used,workers_active,cpu_running,queue_io,queue_cpu,queue_len"
    )
    .unwrap();

    let mut cpu_sum: u64 = 0;
    let mut workers_sum: u64 = 0;
    let mut samples: u64 = 0;
    let mut peak_cpu = 0;
    let mut peak_workers = 0;
    let interval = Duration::from_millis(cfg.monitor_interval_ms);

    while !shutdown.load(Ordering::SeqCst) {
        let snap = scheduler.snapshot();
        let queue_len = snap.queued_io + snap.queued_cpu;
        let elapsed = t0.elapsed().as_millis();

        writeln!(
            file,
            "{},{},{},{},{},{},{}",
            elapsed,
            snap.cpu_budget_used,
            snap.busy_workers,
            snap.cpu_running,
            snap.queued_io,
            snap.queued_cpu,
            queue_len
        )
        .unwrap();

        cpu_sum += snap.cpu_budget_used as u64;
        workers_sum += snap.busy_workers as u64;
        samples += 1;
        peak_cpu = peak_cpu.max(snap.cpu_budget_used);
        peak_workers = peak_workers.max(snap.busy_workers);

        thread::sleep(interval);
    }

    MonitorSummary {
        avg_cpu_budget_used: if samples == 0 { 0.0 } else { cpu_sum as f64 / samples as f64 },
        avg_workers_active: if samples == 0 { 0.0 } else { workers_sum as f64 / samples as f64 },
        peak_cpu_budget_used: peak_cpu,
        peak_workers_active: peak_workers,
        samples,
    }
}

// ─────────────────────────── Metrics ──────────────────────────

fn summarize_results(
    cfg: &Config,
    completions: &[Completion],
    done_count: usize,
    monitor: MonitorSummary,
    t0: Instant,
    total_runtime: Duration,
) {
    let n = completions.len();
    if n == 0 {
        println!("No completions recorded.");
        return;
    }

    let mut wait_sum = 0u128;
    let mut turn_sum = 0u128;
    let mut max_wait = 0u128;
    let mut io_done = 0usize;
    let mut cpu_done = 0usize;
    let mut makespan_end = t0;
    let mut first_id = u64::MAX;
    let mut last_id = 0u64;

    for c in completions {
        let wait = c.start.duration_since(c.arrival).as_millis();
        let turnaround = c.end.duration_since(c.arrival).as_millis();

        wait_sum += wait;
        turn_sum += turnaround;
        max_wait = max_wait.max(wait);
        makespan_end = makespan_end.max(c.end);
        first_id = first_id.min(c.id);
        last_id = last_id.max(c.id);

        match c.kind {
            Kind::Io => io_done += 1,
            Kind::Cpu => cpu_done += 1,
        }
    }

    let makespan = makespan_end.duration_since(t0).as_millis();

    println!();
    println!("── results ──");
    println!("total runtime          : {} ms", total_runtime.as_millis());
    println!("makespan               : {} ms", makespan);
    println!("tasks completed        : {}  (IO={}, CPU={})", done_count, io_done, cpu_done);
    println!("task id range          : {}..={}", first_id, last_id);
    println!("avg wait time          : {:.2} ms", wait_sum as f64 / n as f64);
    println!("avg turnaround time    : {:.2} ms", turn_sum as f64 / n as f64);
    println!("max wait time          : {} ms", max_wait);
    println!("avg CPU budget used    : {:.2} %", monitor.avg_cpu_budget_used);
    println!("peak CPU budget used   : {} %", monitor.peak_cpu_budget_used);
    println!("avg workers active     : {:.2} / {}", monitor.avg_workers_active, cfg.num_workers);
    println!("peak workers active    : {} / {}", monitor.peak_workers_active, cfg.num_workers);
    println!("monitor samples        : {}", monitor.samples);
    println!("monitor csv            : {}", cfg.csv_path);
}

// ─────────────────────────── Main ─────────────────────────────

fn main() {
    let cfg = load_config();
    validate_config(&cfg);

    let io_count = ((cfg.total_tasks as f64) * cfg.io_fraction).round() as usize;
    let cpu_count = cfg.total_tasks - io_count.min(cfg.total_tasks);

    println!("== Optimized scheduler, fixed version ==");
    println!(
        "{} tasks: {} IO / {} CPU, {} workers, simulated CPU cap {}%",
        cfg.total_tasks, io_count, cpu_count, cfg.num_workers, cfg.cpu_cap_pct
    );
    // println!("policy lanes: 8 IO, 6 IO + 1 CPU, or 3 IO + 2 CPU when using 10%/35%/100%");
    //println!("config source: config.toml if present, otherwise built-in defaults");

    let scheduler = Arc::new(Scheduler::new(cfg.clone()));
    let completions = Arc::new(Mutex::new(Vec::with_capacity(cfg.total_tasks)));
    let done_count = Arc::new(AtomicUsize::new(0));
    let shutdown = Arc::new(AtomicBool::new(false));

    let (worker_tx, worker_rx) = mpsc::channel::<Task>();
    let worker_rx: SharedRx = Arc::new(Mutex::new(worker_rx));

    let t0 = Instant::now();

    let mut worker_handles = Vec::with_capacity(cfg.num_workers);
    for id in 0..cfg.num_workers {
        let rx = Arc::clone(&worker_rx);
        let scheduler_for_worker = Arc::clone(&scheduler);
        let comps = Arc::clone(&completions);
        let done = Arc::clone(&done_count);
        worker_handles.push(thread::spawn(move || {
            run_worker(id, rx, scheduler_for_worker, comps, done);
        }));
    }

    let monitor_handle = {
        let monitor_cfg = cfg.clone();
        let scheduler_for_monitor = Arc::clone(&scheduler);
        let shutdown_for_monitor = Arc::clone(&shutdown);
        thread::spawn(move || run_monitor(monitor_cfg, scheduler_for_monitor, shutdown_for_monitor, t0))
    };

    let manager_handle = {
        let scheduler_for_manager = Arc::clone(&scheduler);
        thread::spawn(move || run_manager(scheduler_for_manager, worker_tx))
    };

    let generator_handle = {
        let generator_cfg = cfg.clone();
        let scheduler_for_generator = Arc::clone(&scheduler);
        thread::spawn(move || run_generator(generator_cfg, scheduler_for_generator))
    };

    generator_handle.join().unwrap();
    manager_handle.join().unwrap();

    for handle in worker_handles {
        handle.join().unwrap();
    }

    shutdown.store(true, Ordering::SeqCst);
    let monitor = monitor_handle.join().unwrap();

    let total_runtime = t0.elapsed();
    let comps = completions.lock().unwrap();

    // Sanity check: queue should be drained after manager and workers finish.
    debug_assert_eq!(scheduler.queue_len(), 0);

    summarize_results(
        &cfg,
        &comps,
        done_count.load(Ordering::SeqCst),
        monitor,
        t0,
        total_runtime,
    );
}
