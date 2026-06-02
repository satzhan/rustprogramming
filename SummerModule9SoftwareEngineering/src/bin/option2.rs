// Concurrent Task Dispatcher — Optimized (Simulation 2)
//
// Pipeline (same as option1):
//   Generator  ──tasks──►  Manager (TWO FIFO queues)  ──assign──►  Workers (8)
//                                  │
//                                  └──► Monitor (samples every 10ms ──► CSV)
//
// What changed from option1:
//   • The manager keeps two queues — io_queue and cpu_queue.
//   • The dispatcher picks WHICH queue to pop from based on what's queued.
//
// The dispatch policy is a three-way decision based on the lane geometry
// the cap actually allows:
//
//   8 IO × 10%   = 80%   ✓ 8 lanes  ← when no CPU work is queued
//   6 IO × 10% + 1 CPU × 35% = 95%  ✓ 7 lanes  ← "drain-IO mode"
//   3 IO × 10% + 2 CPU × 35% = 100% ✓ 5 lanes  ← "crunch-CPU mode"
//   2 CPU × 35% = 70%    ✓ 2 lanes  ← BAD: 6 idle workers, avoid this
//   3 CPU × 35% = 105%   ✗ doesn't fit, ever
//
// The rule (per dispatch decision):
//   if no CPU queued      → send IO
//   else if io/cpu ≥ 6    → 6 IO + 1 CPU mode  (drain-IO)
//   else                  → 3 IO + 2 CPU mode  (crunch-CPU)
//
// "6 IO + 1 CPU mode" means: keep at most 1 CPU running concurrently and
// fill the rest with IO. "3 IO + 2 CPU mode" means: keep up to 2 CPUs
// running concurrently — chew through the CPU bottleneck — and fill the
// rest with IO.
//
// CPU is still the critical path: 300 CPU tasks × 200ms / max-2-concurrent
// = 30s minimum. The crunch-CPU mode protects that critical path while
// CPUs are abundant; the drain-IO mode kicks in once the CPU queue is
// thinning and we'd waste lanes running 2-up.

use rand::Rng;
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use serde::Deserialize;
use std::collections::VecDeque;
use std::fs;
use std::io::Write;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use std::time::{Duration, Instant};

// ─────────────────────────── Config ───────────────────────────

#[derive(Debug, Deserialize, Clone)]
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

// ─────────────────────────── Task ─────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind { Io, Cpu }

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
    kind: Kind,
    arrival: Instant,
    start: Instant,
    end: Instant,
}

// ─────────────────── Manager: TWO queues ───────────────────────
//
// One mutex protects BOTH deques together — the dispatch decision needs
// to read both lengths and pop from one of them in a single critical
// section, otherwise a generator push could change the ratio between
// "decide" and "pop" and we'd dispatch from a queue we didn't mean to.
//
// The closed flag is separate (atomic) because only the generator writes
// it and only at the very end.

struct ManagerQueues {
    inner: Mutex<TwoDeques>,
    cv: Condvar,
    closed: AtomicBool,
}

struct TwoDeques {
    io: VecDeque<Task>,
    cpu: VecDeque<Task>,
}

impl ManagerQueues {
    fn new() -> Self {
        Self {
            inner: Mutex::new(TwoDeques {
                io: VecDeque::new(),
                cpu: VecDeque::new(),
            }),
            cv: Condvar::new(),
            closed: AtomicBool::new(false),
        }
    }
    fn push(&self, t: Task) {
        let mut g = self.inner.lock().unwrap();
        match t.kind {
            Kind::Io => g.io.push_back(t),
            Kind::Cpu => g.cpu.push_back(t),
        }
        self.cv.notify_one();
    }
    fn close(&self) {
        self.closed.store(true, Ordering::SeqCst);
        self.cv.notify_all();
    }
    // Block until either we can dispatch *something* (return Some) or
    // both queues are empty AND the generator has closed (return None).
    //
    // The policy decision happens here, atomically with the pop.
    fn pick_next_or_done(&self, cpu_running: usize) -> Option<Task> {
        let mut g = self.inner.lock().unwrap();
        loop {
            // Choose the next task using the three-way rule.
            let pick = decide(&g, cpu_running);
            match pick {
                Pick::Io => {
                    if let Some(t) = g.io.pop_front() { return Some(t); }
                }
                Pick::Cpu => {
                    if let Some(t) = g.cpu.pop_front() { return Some(t); }
                }
                Pick::Wait => {
                    // Either both queues empty, or policy says we should
                    // wait for a worker to finish before dispatching more
                    // (e.g., we're already at the CPU concurrency target).
                    if self.closed.load(Ordering::SeqCst) && g.io.is_empty() && g.cpu.is_empty() {
                        return None;
                    }
                }
            }
            g = self.cv.wait(g).unwrap();
        }
    }
    fn lengths(&self) -> (usize, usize) {
        let g = self.inner.lock().unwrap();
        (g.io.len(), g.cpu.len())
    }
    fn total_len(&self) -> usize {
        let g = self.inner.lock().unwrap();
        g.io.len() + g.cpu.len()
    }
}

// ─────────────────────── Policy ────────────────────────────────
//
// The dispatch rule. Pure function over (queue lengths, what's running).
// `cpu_running` is the number of CPU tasks currently running (we cap CPU
// concurrency at 1 in drain-IO mode, at 2 in crunch-CPU mode).

#[derive(Debug, Clone, Copy)]
enum Pick { Io, Cpu, Wait }

fn decide(g: &TwoDeques, cpu_running: usize) -> Pick {
    let io = g.io.len();
    let cpu = g.cpu.len();

    // Empty case
    if io == 0 && cpu == 0 { return Pick::Wait; }

    // No CPU work pending → just stream IO.
    if cpu == 0 { return Pick::Io; }

    // Decide the mode: drain-IO if io/cpu ≥ 6, else crunch-CPU.
    // Comparing as `io >= 6 * cpu` avoids the divide-by-zero we already
    // ruled out above and stays in integer math.
    let drain_io_mode = io >= 6 * cpu;

    let cpu_target = if drain_io_mode { 1 } else { 2 };

    if cpu_running < cpu_target {
        // We have CPU headroom in our concurrency target. Prefer CPU if
        // there's any queued — that's the bottleneck-protection rule.
        // Fall back to IO if CPU queue is empty (impossible here but
        // kept for clarity).
        if cpu > 0 { Pick::Cpu } else { Pick::Io }
    } else {
        // Already at our CPU concurrency target. Fill remaining lanes
        // with IO if any are queued; otherwise wait for a CPU to finish.
        if io > 0 { Pick::Io } else { Pick::Wait }
    }
}

// ─────────────────── Pool state (unchanged) ────────────────────

struct Counters {
    cpu_used: u32,
    busy_workers: usize,
    cpu_running: usize,   // NEW: how many CPU-kind tasks are in flight
}

struct PoolState {
    inner: Mutex<Counters>,
    cv: Condvar,
    cap: u32,
    n_workers: usize,
}

impl PoolState {
    fn new(cap: u32, n_workers: usize) -> Self {
        Self {
            inner: Mutex::new(Counters { cpu_used: 0, busy_workers: 0, cpu_running: 0 }),
            cv: Condvar::new(),
            cap,
            n_workers,
        }
    }
    // Manager calls this BEFORE handing the task to a worker. Atomic
    // check-and-book on cpu% + slot.
    fn book_for(&self, task: &Task) {
        let mut g = self.inner.lock().unwrap();
        loop {
            let fits_cpu = g.cpu_used + task.cpu_cost <= self.cap;
            let has_worker = g.busy_workers < self.n_workers;
            if fits_cpu && has_worker {
                g.cpu_used += task.cpu_cost;
                g.busy_workers += 1;
                if matches!(task.kind, Kind::Cpu) { g.cpu_running += 1; }
                return;
            }
            g = self.cv.wait(g).unwrap();
        }
    }
    fn release(&self, task: &Task) {
        {
            let mut g = self.inner.lock().unwrap();
            g.cpu_used = g.cpu_used.saturating_sub(task.cpu_cost);
            g.busy_workers = g.busy_workers.saturating_sub(1);
            if matches!(task.kind, Kind::Cpu) { g.cpu_running = g.cpu_running.saturating_sub(1); }
        }
        self.cv.notify_all();
    }
    fn cpu_running(&self) -> usize {
        self.inner.lock().unwrap().cpu_running
    }
    fn snapshot(&self) -> (u32, usize) {
        let g = self.inner.lock().unwrap();
        (g.cpu_used, g.busy_workers)
    }
}

// ─────────────────────── Generator (unchanged shape) ────────────

fn run_generator(cfg: Config, queues: Arc<ManagerQueues>) {
    let mut rng = ChaCha8Rng::seed_from_u64(cfg.rng_seed);
    let interval = Duration::from_millis(cfg.arrival_interval_ms);

    for id in 0..cfg.total_tasks as u64 {
        let kind = if rng.gen::<f64>() < cfg.io_fraction { Kind::Io } else { Kind::Cpu };
        let cpu_cost = match kind {
            Kind::Io => cfg.io_cpu_pct,
            Kind::Cpu => cfg.cpu_cpu_pct,
        };
        let task = Task {
            id, kind, cpu_cost,
            duration_ms: cfg.task_duration_ms,
            arrival: Instant::now(),
        };
        queues.push(task);
        thread::sleep(interval);
    }
    queues.close();
}

// ─────────────────────── Manager (new logic) ───────────────────
//
// Same shape as option1's manager but the queue choice is policy-driven.
//
// Note the coupling between PoolState and ManagerQueues: when a worker
// finishes, it notifies PoolState's condvar (so book_for wakes), but the
// manager is sleeping on ManagerQueues' condvar. We close that gap by
// also notifying ManagerQueues' condvar at the right moments. The
// simplest place is here in the manager loop: after book_for() returns
// (a worker just finished, or there was room already), we go fetch
// another task.
//
// We *also* need the manager to wake when the policy choice changes,
// e.g., a CPU task finishes → cpu_running drops → we might now be allowed
// to dispatch another CPU. Easiest fix: have the worker's release() also
// notify the manager. We pass an Arc to the queue's condvar into the
// worker.

fn run_manager(
    queues: Arc<ManagerQueues>,
    pool: Arc<PoolState>,
    worker_tx: std::sync::mpsc::Sender<Task>,
) {
    loop {
        let cpu_running = pool.cpu_running();
        let task = match queues.pick_next_or_done(cpu_running) {
            Some(t) => t,
            None => break,
        };
        // Book CPU + slot atomically against the cap, then send.
        pool.book_for(&task);
        // println!("DISPATCH kind={:?} cpu_running={} io_q={} cpu_q={} cpu%={} busy={}",
        //     task.kind, pool.cpu_running(),
        //     queues.lengths().0, queues.lengths().1,
        //     pool.snapshot().0, pool.snapshot().1);
        worker_tx.send(task).expect("worker channel closed");
    }
    drop(worker_tx);
}

// ─────────────────────── Worker pool ───────────────────────────

type SharedRx = Arc<Mutex<std::sync::mpsc::Receiver<Task>>>;

fn run_worker(
    _id: usize,
    rx: SharedRx,
    pool: Arc<PoolState>,
    queues: Arc<ManagerQueues>,   // ← added: so we can wake the manager on release
    completions: Arc<Mutex<Vec<Completion>>>,
    done_count: Arc<AtomicUsize>,
) {
    loop {
        let task = {
            let guard = rx.lock().unwrap();
            match guard.recv() {
                Ok(t) => t,
                Err(_) => return,
            }
        };
        let start = Instant::now();
        thread::sleep(Duration::from_millis(task.duration_ms));
        let end = Instant::now();
        pool.release(&task);
        // Wake the manager: a finished CPU task may have changed the
        // policy decision (cpu_running just dropped).
        queues.cv.notify_all();

        completions.lock().unwrap().push(Completion {
            kind: task.kind,
            arrival: task.arrival,
            start, end,
        });
        done_count.fetch_add(1, Ordering::SeqCst);
    }
}

// ─────────────────────── Monitor ───────────────────────────────
//
// Same CSV shape as option1 so plot_monitor.py works on both runs.

fn run_monitor(
    cfg: Config,
    queues: Arc<ManagerQueues>,
    pool: Arc<PoolState>,
    shutdown: Arc<AtomicBool>,
    t0: Instant,
) -> (f64, f64, u64) {
    let mut file = fs::File::create(&cfg.csv_path).expect("cannot create CSV");
    writeln!(file, "elapsed_ms,cpu_pct,workers_active,queue_len").unwrap();

    let mut cpu_sum: u64 = 0;
    let mut workers_sum: u64 = 0;
    let mut samples: u64 = 0;
    let interval = Duration::from_millis(cfg.monitor_interval_ms);

    while !shutdown.load(Ordering::SeqCst) {
        let (cpu, busy) = pool.snapshot();
        let qlen = queues.total_len();
        let elapsed = t0.elapsed().as_millis();
        writeln!(file, "{},{},{},{}", elapsed, cpu, busy, qlen).unwrap();
        cpu_sum += cpu as u64;
        workers_sum += busy as u64;
        samples += 1;
        thread::sleep(interval);
    }
    let avg_cpu = if samples == 0 { 0.0 } else { cpu_sum as f64 / samples as f64 };
    let avg_workers = if samples == 0 { 0.0 } else { workers_sum as f64 / samples as f64 };
    (avg_cpu, avg_workers, samples)
}

// ─────────────────────── Main ──────────────────────────────────

fn main() {
    let cfg_text = fs::read_to_string("config.toml").expect("config.toml missing");
    let mut cfg: Config = toml::from_str(&cfg_text).expect("config.toml is malformed");
    // Default CSV name for option2 (don't clobber option1's monitor_log.csv).
    if cfg.csv_path == "monitor_log.csv" {
        cfg.csv_path = "monitor_log_option2.csv".to_string();
    }

    println!("== Optimized simulation (option2) ==");
    // println!("policy: 2-queue, ratio-based mode switch");
    // println!("  no CPU queued     → 8 IO mode");
    // println!("  io/cpu ≥ 6        → 6 IO + 1 CPU mode  (drain-IO)");
    // println!("  otherwise         → 3 IO + 2 CPU mode  (crunch-CPU)");
    println!("{} tasks, {:.0}% IO / {:.0}% CPU, {} workers, cap {}%",
        cfg.total_tasks,
        cfg.io_fraction * 100.0,
        (1.0 - cfg.io_fraction) * 100.0,
        cfg.num_workers, cfg.cpu_cap_pct);

    let queues = Arc::new(ManagerQueues::new());
    let pool = Arc::new(PoolState::new(cfg.cpu_cap_pct, cfg.num_workers));
    let completions = Arc::new(Mutex::new(Vec::with_capacity(cfg.total_tasks)));
    let done_count = Arc::new(AtomicUsize::new(0));
    let shutdown = Arc::new(AtomicBool::new(false));

    let (worker_tx, worker_rx) = std::sync::mpsc::channel::<Task>();
    let worker_rx: SharedRx = Arc::new(Mutex::new(worker_rx));

    let t0 = Instant::now();

    // Workers
    let mut worker_handles = Vec::with_capacity(cfg.num_workers);
    for id in 0..cfg.num_workers {
        let rx = Arc::clone(&worker_rx);
        let pool = Arc::clone(&pool);
        let queues = Arc::clone(&queues);
        let comps = Arc::clone(&completions);
        let dc = Arc::clone(&done_count);
        worker_handles.push(thread::spawn(move || {
            run_worker(id, rx, pool, queues, comps, dc);
        }));
    }

    // Monitor
    let mon_cfg = cfg.clone();
    let mon_q = Arc::clone(&queues);
    let mon_pool = Arc::clone(&pool);
    let mon_shutdown = Arc::clone(&shutdown);
    let monitor_handle = thread::spawn(move || run_monitor(mon_cfg, mon_q, mon_pool, mon_shutdown, t0));

    // Manager
    let mgr_q = Arc::clone(&queues);
    let mgr_pool = Arc::clone(&pool);
    let manager_handle = thread::spawn(move || run_manager(mgr_q, mgr_pool, worker_tx));

    // Generator
    let gen_cfg = cfg.clone();
    let gen_q = Arc::clone(&queues);
    let generator_handle = thread::spawn(move || run_generator(gen_cfg, gen_q));

    // Shutdown order: generator → manager → workers → monitor
    generator_handle.join().unwrap();
    manager_handle.join().unwrap();
    for h in worker_handles { h.join().unwrap(); }
    shutdown.store(true, Ordering::SeqCst);
    let (avg_cpu, avg_workers, samples) = monitor_handle.join().unwrap();

    let total_runtime = t0.elapsed();

    let comps = completions.lock().unwrap();
    let n = comps.len() as f64;
    let (mut wait_sum, mut turn_sum, mut max_wait) = (0u128, 0u128, 0u128);
    let (mut io_done, mut cpu_done) = (0u64, 0u64);
    let mut makespan_end = t0;
    for c in comps.iter() {
        let wait = c.start.duration_since(c.arrival).as_millis();
        let turn = c.end.duration_since(c.arrival).as_millis();
        wait_sum += wait;
        turn_sum += turn;
        if wait > max_wait { max_wait = wait; }
        if c.end > makespan_end { makespan_end = c.end; }
        match c.kind { Kind::Io => io_done += 1, Kind::Cpu => cpu_done += 1 }
    }
    let makespan = makespan_end.duration_since(t0).as_millis();

    println!();
    println!("── results ──");
    println!("total runtime         : {} ms", total_runtime.as_millis());
    println!("makespan              : {} ms", makespan);
    println!("tasks completed       : {}  (IO={}, CPU={})",
        done_count.load(Ordering::SeqCst), io_done, cpu_done);
    println!("avg wait time         : {:.2} ms", wait_sum as f64 / n);
    println!("avg turnaround time   : {:.2} ms", turn_sum as f64 / n);
    println!("max wait time         : {} ms", max_wait);
    println!("avg CPU usage         : {:.2} %", avg_cpu);
    println!("avg workers active    : {:.2} / {}", avg_workers, cfg.num_workers);
    println!("monitor samples       : {}", samples);
    println!("monitor csv           : {}", cfg.csv_path);
}