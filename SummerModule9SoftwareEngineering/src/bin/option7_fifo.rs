// Concurrent Task Dispatcher — FIFO baseline (Simulation 1)
//
// Pipeline:
//   Generator  ──tasks──►  Manager (FIFO queue)  ──assign──►  Worker pool (8)
//                                  │
//                                  └──► Monitor (samples every 10ms ──► CSV)
//
// Pure FIFO: the manager only dispatches the head of the queue. If the head
// can't fit (no free worker, or CPU headroom < task's cost), the manager
// waits. No skip-ahead. No re-queueing.
//
// Workers own the global CPU counter — they add the task's cost when they
// start, sleep for the task duration, then subtract when done.

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
enum Kind {
    Io,
    Cpu,
}

#[derive(Debug, Clone, Copy)]
struct Task {
    id: u64,
    kind: Kind,
    cpu_cost: u32,        // % of global CPU this task consumes while running
    duration_ms: u64,
    arrival: Instant,     // stamped by the generator when the task is created
}

// per-task timing the worker fills in for the final summary
#[derive(Debug, Clone, Copy)]
struct Completion {
    kind: Kind,
    arrival: Instant,
    start: Instant,
    end: Instant,
}

// ─────────────────────── Shared state ─────────────────────────
//
// Two pieces of mutable shared state, both behind their own lock so
// you can point at each one and say what it protects:
//
//   1. ManagerQueue  — the FIFO of pending tasks
//   2. PoolState     — global CPU usage + count of busy workers
//
// A Condvar on PoolState lets the manager sleep until headroom appears
// instead of busy-waiting.

struct ManagerQueue {
    inner: Mutex<VecDeque<Task>>,
    cv: Condvar,
    closed: AtomicBool,    // generator sets this when it's done sending
    io_count: AtomicUsize, // tasks of Kind::Io currently in the queue
    cpu_count: AtomicUsize,// tasks of Kind::Cpu currently in the queue
}

impl ManagerQueue {
    fn new() -> Self {
        Self {
            inner: Mutex::new(VecDeque::new()),
            cv: Condvar::new(),
            closed: AtomicBool::new(false),
            io_count: AtomicUsize::new(0),
            cpu_count: AtomicUsize::new(0),
        }
    }
    fn push(&self, t: Task) {
        match t.kind {
            Kind::Io  => self.io_count.fetch_add(1, Ordering::Relaxed),
            Kind::Cpu => self.cpu_count.fetch_add(1, Ordering::Relaxed),
        };
        self.inner.lock().unwrap().push_back(t);
        self.cv.notify_one();
    }
    fn close(&self) {
        self.closed.store(true, Ordering::SeqCst);
        self.cv.notify_all();
    }
    // Block until either a task is available (return Some) or the queue
    // is closed AND empty (return None — manager can shut down).
    fn pop_head_or_done(&self) -> Option<Task> {
        let mut q = self.inner.lock().unwrap();
        loop {
            if let Some(t) = q.pop_front() {
                match t.kind {
                    Kind::Io  => self.io_count.fetch_sub(1, Ordering::Relaxed),
                    Kind::Cpu => self.cpu_count.fetch_sub(1, Ordering::Relaxed),
                };
                return Some(t);
            }
            if self.closed.load(Ordering::SeqCst) {
                return None;
            }
            q = self.cv.wait(q).unwrap();
        }
    }
    // O(1) snapshot for the monitor — no lock, no iteration.
    fn lens_by_kind(&self) -> (usize, usize) {
        (
            self.io_count.load(Ordering::Relaxed),
            self.cpu_count.load(Ordering::Relaxed),
        )
    }
}

// One mutex protects BOTH counters together. That's the whole point of the
// fix: the manager has to "see CPU & slot" and "book CPU & slot" inside the
// same critical section, otherwise two dispatch decisions can both think
// they have room and oversubscribe the cap. (TOCTOU bug, classic.)
struct Counters {
    cpu_used: u32,        // current global CPU%
    busy_workers: usize,  // workers currently running a task
}

struct PoolState {
    inner: Mutex<Counters>,
    cv: Condvar,          // notified whenever a worker finishes
    cap: u32,
    n_workers: usize,
}

impl PoolState {
    fn new(cap: u32, n_workers: usize) -> Self {
        Self {
            inner: Mutex::new(Counters { cpu_used: 0, busy_workers: 0 }),
            cv: Condvar::new(),
            cap,
            n_workers,
        }
    }
    // Manager calls this BEFORE handing the task to a worker. It blocks
    // until the head task fits, then atomically books CPU + slot. Pure FIFO.
    //
    // Booking happens at dispatch time (here) — that's what makes the cap
    // actually hold. The worker's job is now just to release on finish.
    fn book_for(&self, task_cost: u32) {
        let mut g = self.inner.lock().unwrap();
        loop {
            let fits_cpu = g.cpu_used + task_cost <= self.cap;
            let has_worker = g.busy_workers < self.n_workers;
            if fits_cpu && has_worker {
                g.cpu_used += task_cost;
                g.busy_workers += 1;
                return;
            }
            g = self.cv.wait(g).unwrap();
        }
    }
    // Worker calls this when its task is done — release CPU + slot together.
    fn release(&self, task_cost: u32) {
        {
            let mut g = self.inner.lock().unwrap();
            g.cpu_used = g.cpu_used.saturating_sub(task_cost);
            g.busy_workers = g.busy_workers.saturating_sub(1);
        }
        self.cv.notify_all();
    }
    fn snapshot(&self) -> (u32, usize) {
        let g = self.inner.lock().unwrap();
        (g.cpu_used, g.busy_workers)
    }
}

// ─────────────────────── Generator ────────────────────────────
//
// Builds `total_tasks` tasks with the configured IO/CPU mix (interleaved
// random order, seeded), and pushes them into the manager queue at fixed
// intervals to simulate arrival over time.

fn run_generator(cfg: Config, queue: Arc<ManagerQueue>) {
    let mut rng = ChaCha8Rng::seed_from_u64(cfg.rng_seed);
    let interval = Duration::from_millis(cfg.arrival_interval_ms);

    for id in 0..cfg.total_tasks as u64 {
        let kind = if rng.gen::<f64>() < cfg.io_fraction {
            Kind::Io
        } else {
            Kind::Cpu
        };
        let cpu_cost = match kind {
            Kind::Io => cfg.io_cpu_pct,
            Kind::Cpu => cfg.cpu_cpu_pct,
        };
        let task = Task {
            id,
            kind,
            cpu_cost,
            duration_ms: cfg.task_duration_ms,
            arrival: Instant::now(),
        };
        queue.push(task);
        thread::sleep(interval);
    }
    queue.close(); // signals manager: no more tasks coming
}

// ─────────────────────── Manager ──────────────────────────────
//
// One thread. Pulls tasks off the FIFO in order, blocks until the head
// fits (free worker + CPU headroom), then sends to a worker via mpsc.

fn run_manager(
    queue: Arc<ManagerQueue>,
    pool: Arc<PoolState>,
    worker_tx: std::sync::mpsc::Sender<Task>,
) {
    while let Some(task) = queue.pop_head_or_done() {
        // Pure FIFO: wait until THIS task fits, then book CPU + slot atomically.
        // No skip-ahead. By the time send() runs, the resources are already
        // reserved against the global cap.
        pool.book_for(task.cpu_cost);
        worker_tx.send(task).expect("worker channel closed");
    }
    drop(worker_tx); // closing the channel tells workers to exit
}

// ─────────────────────── Worker pool ──────────────────────────
//
// N worker threads share one mpsc receiver (behind a Mutex). Each worker:
//   1. recv() a task                (blocks)
//   2. sleep for task.duration_ms   (simulates the work)
//   3. release CPU + slot           (notifies the manager)
//   4. record completion
//
// Note: CPU is *booked* by the manager at dispatch (see PoolState::book_for).
// The worker only releases. This is what guarantees the global cap holds —
// otherwise the manager could green-light multiple tasks against the same
// stale CPU reading before any worker had time to add its cost.

type SharedRx = Arc<Mutex<std::sync::mpsc::Receiver<Task>>>;

fn run_worker(
    _id: usize,
    rx: SharedRx,
    pool: Arc<PoolState>,
    completions: Arc<Mutex<Vec<Completion>>>,
    done_count: Arc<AtomicUsize>,
) {
    loop {
        // Lock the receiver only long enough to recv() one task.
        let task = {
            let guard = rx.lock().unwrap();
            match guard.recv() {
                Ok(t) => t,
                Err(_) => return, // channel closed by manager
            }
        };
        let start = Instant::now();

        thread::sleep(Duration::from_millis(task.duration_ms));

        let end = Instant::now();
        pool.release(task.cpu_cost);

        completions.lock().unwrap().push(Completion {
            kind: task.kind,
            arrival: task.arrival,
            start,
            end,
        });
        done_count.fetch_add(1, Ordering::SeqCst);
    }
}

// ─────────────────────── Monitor ──────────────────────────────
//
// Independent thread. Every `monitor_interval_ms`, samples (cpu%, busy
// workers, queue length) and writes a CSV row. Stops when the main
// thread sets the shutdown flag.

fn run_monitor(
    cfg: Config,
    queue: Arc<ManagerQueue>,
    pool: Arc<PoolState>,
    shutdown: Arc<AtomicBool>,
    t0: Instant,
) -> (f64, f64, u64) {
    // Returns (avg_cpu_pct, avg_workers_active, samples_taken).
    let mut file = fs::File::create(&cfg.csv_path).expect("cannot create CSV");
    writeln!(file, "elapsed_ms,cpu_pct,workers_active,queue_io,queue_cpu,queue_total").unwrap();

    let mut cpu_sum: u64 = 0;
    let mut workers_sum: u64 = 0;
    let mut samples: u64 = 0;
    let interval = Duration::from_millis(cfg.monitor_interval_ms);

    while !shutdown.load(Ordering::SeqCst) {
        let (cpu, busy) = pool.snapshot();
        let (q_io, q_cpu) = queue.lens_by_kind();
        let q_total = q_io + q_cpu;
        let elapsed = t0.elapsed().as_millis();
        writeln!(file, "{},{},{},{},{},{}", elapsed, cpu, busy, q_io, q_cpu, q_total).unwrap();
        cpu_sum += cpu as u64;
        workers_sum += busy as u64;
        samples += 1;
        thread::sleep(interval);
    }
    let avg_cpu = if samples == 0 { 0.0 } else { cpu_sum as f64 / samples as f64 };
    let avg_workers = if samples == 0 { 0.0 } else { workers_sum as f64 / samples as f64 };
    (avg_cpu, avg_workers, samples)
}

// ─────────────────────── Main ─────────────────────────────────

fn main() {
    // Load config
    let cfg_text = fs::read_to_string("config.toml").expect("config.toml missing");
    let cfg: Config = toml::from_str(&cfg_text).expect("config.toml is malformed");
    println!("== FIFO simulation ==");
    println!("{} tasks, {:.0}% IO / {:.0}% CPU, {} workers, cap {}%",
        cfg.total_tasks,
        cfg.io_fraction * 100.0,
        (1.0 - cfg.io_fraction) * 100.0,
        cfg.num_workers,
        cfg.cpu_cap_pct,
    );

    // Shared state
    let queue = Arc::new(ManagerQueue::new());
    let pool = Arc::new(PoolState::new(cfg.cpu_cap_pct, cfg.num_workers));
    let completions = Arc::new(Mutex::new(Vec::with_capacity(cfg.total_tasks)));
    let done_count = Arc::new(AtomicUsize::new(0));
    let shutdown = Arc::new(AtomicBool::new(false));

    // Manager → workers channel (single producer = manager, multi-consumer
    // = workers, so we wrap the receiver in an Arc<Mutex<_>>).
    let (worker_tx, worker_rx) = std::sync::mpsc::channel::<Task>();
    let worker_rx: SharedRx = Arc::new(Mutex::new(worker_rx));

    let t0 = Instant::now();

    // Spawn workers
    let mut worker_handles = Vec::with_capacity(cfg.num_workers);
    for id in 0..cfg.num_workers {
        let rx = Arc::clone(&worker_rx);
        let pool = Arc::clone(&pool);
        let comps = Arc::clone(&completions);
        let dc = Arc::clone(&done_count);
        worker_handles.push(thread::spawn(move || {
            run_worker(id, rx, pool, comps, dc);
        }));
    }

    // Spawn monitor (returns averages on join)
    let mon_cfg = cfg.clone();
    let mon_q = Arc::clone(&queue);
    let mon_pool = Arc::clone(&pool);
    let mon_shutdown = Arc::clone(&shutdown);
    let monitor_handle = thread::spawn(move || {
        run_monitor(mon_cfg, mon_q, mon_pool, mon_shutdown, t0)
    });

    // Spawn manager
    let mgr_q = Arc::clone(&queue);
    let mgr_pool = Arc::clone(&pool);
    let manager_handle = thread::spawn(move || {
        run_manager(mgr_q, mgr_pool, worker_tx);
    });

    // Spawn generator
    let gen_cfg = cfg.clone();
    let gen_q = Arc::clone(&queue);
    let generator_handle = thread::spawn(move || {
        run_generator(gen_cfg, gen_q);
    });

    // Shutdown sequence — each step waits for the previous to finish:
    //   1. generator stops sending and closes the queue
    //   2. manager drains the queue and drops worker_tx
    //   3. workers see channel closed, exit
    //   4. monitor flag is flipped, monitor exits
    generator_handle.join().unwrap();
    manager_handle.join().unwrap();
    for h in worker_handles { h.join().unwrap(); }
    shutdown.store(true, Ordering::SeqCst);
    let (avg_cpu, avg_workers, samples) = monitor_handle.join().unwrap();

    let total_runtime = t0.elapsed();

    // ── Final summary ─────────────────────────────────────────
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