// Concurrent Task Dispatcher — Mode-Batch Two-Queue Scheduler (Simulation 3)
//
// Pipeline:
//   Generator  ──tasks──►  Manager (IO queue + CPU queue)  ──batch assign──►  Worker pool
//                                  │
//                                  └──► Monitor (samples every 10ms ──► CSV)
//
// Difference from Simulation 1:
//   Simulation 1 uses one FIFO queue. If the head task cannot fit, everything
//   behind it waits too.
//
// Difference from Simulation 2:
//   Simulation 2 chooses one runnable task at a time using queue pressure.
//   This version chooses a BATCH MODE when possible:
//       - 8 IO
//       - 6 IO + 1 CPU
//       - 3 IO + 2 CPU
//
// Why these modes?
//   For a 700 IO / 300 CPU workload, the target long-run mix is 70% IO / 30% CPU.
//   The available modes have these mixes:
//       8 IO          = 100% IO
//       6 IO + 1 CPU  ≈ 85.7% IO
//       3 IO + 2 CPU  = 60% IO
//   The target 70% sits between the second and third modes. The scheduler
//   therefore uses cumulative dispatched ratio as its compass: it picks the
//   mode that moves the already-dispatched mix closest to cfg.io_fraction.
//
// Important simulation caveat:
//   The math recipe assumes all 700/300 tasks are already waiting. This program
//   has arrivals over time, CPU caps, and worker limits. So the recipe becomes
//   a dynamic policy: pick the best full mode that is currently queued AND fits
//   current resources. If no full mode fits yet, use a smaller fallback batch so
//   workers do not sit idle unnecessarily.
//
// Output is intentionally kept compatible with Simulation 1 and Simulation 2:
//   - same CSV columns: elapsed_ms,cpu_pct,workers_active,queue_len
//   - same final report lines
//
// Workers do NOT decide scheduling. The manager books CPU + worker slots before
// sending tasks, so the cap is enforced at dispatch time.

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
    cpu_cost: u32,    // % of global CPU this task consumes while running
    duration_ms: u64,
    arrival: Instant, // stamped by the generator when the task is created
}

// Per-task timing the worker fills in for the final summary.
#[derive(Debug, Clone, Copy)]
struct Completion {
    kind: Kind,
    arrival: Instant,
    start: Instant,
    end: Instant,
}

// ─────────────────────── Dispatch modes ───────────────────────

#[derive(Debug, Clone, Copy)]
struct DispatchMode {
    name: &'static str,
    io: usize,
    cpu: usize,
}

impl DispatchMode {
    fn total(self) -> usize {
        self.io + self.cpu
    }

    fn cost(self, io_cost: u32, cpu_cost: u32) -> u32 {
        (self.io as u32) * io_cost + (self.cpu as u32) * cpu_cost
    }
}

const DISPATCH_MODES: [DispatchMode; 3] = [
    DispatchMode {
        name: "8 IO",
        io: 8,
        cpu: 0,
    },
    DispatchMode {
        name: "6 IO + 1 CPU",
        io: 6,
        cpu: 1,
    },
    DispatchMode {
        name: "3 IO + 2 CPU",
        io: 3,
        cpu: 2,
    },
];

fn ratio_error(io_count: usize, cpu_count: usize, target_io_fraction: f64) -> f64 {
    let total = io_count + cpu_count;
    if total == 0 {
        return 0.0;
    }
    let target = target_io_fraction.clamp(0.0, 1.0);
    let actual = io_count as f64 / total as f64;
    (actual - target).abs()
}

// ─────────────────────── Scheduler signal ─────────────────────
//
// The manager needs to wake up for TWO kinds of events:
//   1. A new task arrived in either queue.
//   2. A worker finished and released CPU/worker capacity.
//
// Instead of trying to wait on two unrelated Condvars, we use one small
// shared "something changed" signal. Both ManagerQueue::push/close and
// PoolState::release notify it.

struct SchedulerSignal {
    version: Mutex<u64>,
    cv: Condvar,
}

impl SchedulerSignal {
    fn new() -> Self {
        Self {
            version: Mutex::new(0),
            cv: Condvar::new(),
        }
    }

    fn version(&self) -> u64 {
        *self.version.lock().unwrap()
    }

    fn notify_change(&self) {
        let mut v = self.version.lock().unwrap();
        *v += 1;
        self.cv.notify_all();
    }

    fn wait_for_change(&self, last_seen: u64) -> u64 {
        let mut v = self.version.lock().unwrap();
        while *v == last_seen {
            v = self.cv.wait(v).unwrap();
        }
        *v
    }
}

// ─────────────────────── Shared queue state ───────────────────
//
// ManagerQueue protects TWO FIFO queues. We are no longer globally FIFO.
// We are FIFO only inside each task kind.
//
// Teaching contrast:
//   - One FIFO queue is simple and fair by arrival order, but can block.
//   - Two queues are less globally fair, but can use capacity more intelligently.
//   - Mode batching adds a third idea: scheduling can aim at a long-term ratio.

struct QueueInner {
    io: VecDeque<Task>,
    cpu: VecDeque<Task>,
    closed: bool,
}

struct ManagerQueue {
    inner: Mutex<QueueInner>,
    signal: Arc<SchedulerSignal>,
}

impl ManagerQueue {
    fn new(signal: Arc<SchedulerSignal>) -> Self {
        Self {
            inner: Mutex::new(QueueInner {
                io: VecDeque::new(),
                cpu: VecDeque::new(),
                closed: false,
            }),
            signal,
        }
    }

    fn push(&self, t: Task) {
        {
            let mut q = self.inner.lock().unwrap();
            match t.kind {
                Kind::Io => q.io.push_back(t),
                Kind::Cpu => q.cpu.push_back(t),
            }
        }
        self.signal.notify_change();
    }

    fn close(&self) {
        {
            let mut q = self.inner.lock().unwrap();
            q.closed = true;
        }
        self.signal.notify_change();
    }

    fn len(&self) -> usize {
        let q = self.inner.lock().unwrap();
        q.io.len() + q.cpu.len()
    }

    fn is_closed_and_empty(&self) -> bool {
        let q = self.inner.lock().unwrap();
        q.closed && q.io.is_empty() && q.cpu.is_empty()
    }

    fn pop_n(queue: &mut VecDeque<Task>, n: usize, out: &mut Vec<Task>) {
        for _ in 0..n {
            out.push(queue.pop_front().expect("queue length checked before pop"));
        }
    }

    // Try to choose a batch that fits right now.
    //
    // First preference: a full dispatch mode from DISPATCH_MODES.
    // Fallback: a smaller greedy batch, one task at a time, so workers do not
    // unnecessarily idle while the manager waits for a perfect full mode.
    //
    // This method does NOT wait. Waiting is handled by run_manager using
    // SchedulerSignal, because the manager may need to wake up for either a
    // task-arrival event or a resource-release event.
    #[allow(clippy::too_many_arguments)]
    fn pop_best_batch_that_fits(
        &self,
        cpu_room: u32,
        free_workers: usize,
        target_io_fraction: f64,
        io_cost: u32,
        cpu_cost: u32,
        dispatched_io: usize,
        dispatched_cpu: usize,
    ) -> Option<Vec<Task>> {
        if free_workers == 0 {
            return None;
        }

        let mut q = self.inner.lock().unwrap();

        // ── 1) Full-mode attempt ───────────────────────────────
        // Among modes that are currently possible, choose the one that makes
        // cumulative dispatched ratio closest to the target IO fraction.
        let mut best_mode: Option<DispatchMode> = None;
        let mut best_error = f64::INFINITY;
        let mut best_size = 0usize;

        for mode in DISPATCH_MODES {
            let enough_tasks = q.io.len() >= mode.io && q.cpu.len() >= mode.cpu;
            let enough_workers = mode.total() <= free_workers;
            let enough_cpu = mode.cost(io_cost, cpu_cost) <= cpu_room;

            if !(enough_tasks && enough_workers && enough_cpu) {
                continue;
            }

            let next_io = dispatched_io + mode.io;
            let next_cpu = dispatched_cpu + mode.cpu;
            let err = ratio_error(next_io, next_cpu, target_io_fraction);

            // Primary: ratio closeness. Secondary: larger batch, because the
            // original math minimizes number of pick rounds.
            if err < best_error - 1e-12
                || ((err - best_error).abs() <= 1e-12 && mode.total() > best_size)
            {
                best_mode = Some(mode);
                best_error = err;
                best_size = mode.total();
            }
        }

        if let Some(mode) = best_mode {
            let mut batch = Vec::with_capacity(mode.total());
            // Sending IO first keeps each queue FIFO internally. If you want a
            // more visual classroom trace, change this to interleave IO/CPU.
            Self::pop_n(&mut q.io, mode.io, &mut batch);
            Self::pop_n(&mut q.cpu, mode.cpu, &mut batch);
            return Some(batch);
        }

        // ── 2) Fallback partial batch ───────────────────────────
        // This is what makes the policy practical under streaming arrivals.
        // Example: if only 2 IO tasks have arrived so far, waiting for 8 IO
        // would make the optimized scheduler artificially worse.
        let mut batch = Vec::new();
        let mut remaining_workers = free_workers;
        let mut remaining_cpu_room = cpu_room;
        let mut tmp_io = dispatched_io;
        let mut tmp_cpu = dispatched_cpu;

        while remaining_workers > 0 {
            let io_fits = q
                .io
                .front()
                .map(|task| task.cpu_cost <= remaining_cpu_room)
                .unwrap_or(false);
            let cpu_fits = q
                .cpu
                .front()
                .map(|task| task.cpu_cost <= remaining_cpu_room)
                .unwrap_or(false);

            let pick = match (io_fits, cpu_fits) {
                (false, false) => None,
                (true, false) => Some(Kind::Io),
                (false, true) => Some(Kind::Cpu),
                (true, true) => {
                    let err_if_io = ratio_error(tmp_io + 1, tmp_cpu, target_io_fraction);
                    let err_if_cpu = ratio_error(tmp_io, tmp_cpu + 1, target_io_fraction);
                    if err_if_io <= err_if_cpu {
                        Some(Kind::Io)
                    } else {
                        Some(Kind::Cpu)
                    }
                }
            };

            let Some(kind) = pick else {
                break;
            };

            let task = match kind {
                Kind::Io => q.io.pop_front().expect("IO task checked before pop"),
                Kind::Cpu => q.cpu.pop_front().expect("CPU task checked before pop"),
            };

            remaining_cpu_room = remaining_cpu_room.saturating_sub(task.cpu_cost);
            remaining_workers -= 1;
            match kind {
                Kind::Io => tmp_io += 1,
                Kind::Cpu => tmp_cpu += 1,
            }
            batch.push(task);
        }

        if batch.is_empty() {
            None
        } else {
            Some(batch)
        }
    }
}

// One mutex protects BOTH counters together. The manager must "see capacity"
// and "book capacity" as one decision. Otherwise two dispatch decisions could
// both think they have room and oversubscribe the CPU cap.

struct Counters {
    cpu_used: u32,        // current global CPU%
    busy_workers: usize, // workers currently running a task
}

struct PoolState {
    inner: Mutex<Counters>,
    signal: Arc<SchedulerSignal>,
    cap: u32,
    n_workers: usize,
}

impl PoolState {
    fn new(cap: u32, n_workers: usize, signal: Arc<SchedulerSignal>) -> Self {
        Self {
            inner: Mutex::new(Counters {
                cpu_used: 0,
                busy_workers: 0,
            }),
            signal,
            cap,
            n_workers,
        }
    }

    fn available(&self) -> (u32, usize) {
        let g = self.inner.lock().unwrap();
        let cpu_room = self.cap.saturating_sub(g.cpu_used);
        let free_workers = self.n_workers.saturating_sub(g.busy_workers);
        (cpu_room, free_workers)
    }

    // Manager calls this BEFORE handing tasks to workers.
    // There is exactly one manager, so if a batch fit when selected, no other
    // manager can steal that capacity before this booking happens.
    fn book_batch(&self, total_cost: u32, task_count: usize) {
        let mut g = self.inner.lock().unwrap();
        let fits_cpu = total_cost <= self.cap.saturating_sub(g.cpu_used);
        let has_workers = task_count <= self.n_workers.saturating_sub(g.busy_workers);

        assert!(
            fits_cpu && has_workers,
            "internal scheduler error: selected a batch that did not actually fit"
        );

        g.cpu_used = g.cpu_used.saturating_add(total_cost);
        g.busy_workers += task_count;
    }

    // Worker calls this when its task is done — release CPU + slot together.
    fn release(&self, task_cost: u32) {
        {
            let mut g = self.inner.lock().unwrap();
            g.cpu_used = g.cpu_used.saturating_sub(task_cost);
            g.busy_workers = g.busy_workers.saturating_sub(1);
        }
        self.signal.notify_change();
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
        // Rust 2024 reserves `gen` as a keyword, so use gen_range instead of
        // rng.gen::<f64>().
        let kind = if rng.gen_range(0.0..1.0) < cfg.io_fraction {
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
// One thread. Looks at both queues, picks a runnable batch, books resources,
// then sends each task in that batch to the worker pool.
//
// This manager is optimized around the queue logic:
//   - It avoids FIFO head-of-line blocking.
//   - It keeps FIFO ordering within IO and within CPU.
//   - It tries to keep the long-term dispatched mix close to cfg.io_fraction.

fn run_manager(
    cfg: Config,
    queue: Arc<ManagerQueue>,
    pool: Arc<PoolState>,
    signal: Arc<SchedulerSignal>,
    worker_tx: std::sync::mpsc::Sender<Task>,
) {
    let mut seen = signal.version();
    let mut dispatched_io = 0usize;
    let mut dispatched_cpu = 0usize;

    loop {
        let (cpu_room, free_workers) = pool.available();

        if let Some(batch) = queue.pop_best_batch_that_fits(
            cpu_room,
            free_workers,
            cfg.io_fraction,
            cfg.io_cpu_pct,
            cfg.cpu_cpu_pct,
            dispatched_io,
            dispatched_cpu,
        ) {
            let batch_cost = batch.iter().map(|task| task.cpu_cost).sum::<u32>();
            let batch_len = batch.len();

            // Book all resources for the batch before any worker receives a task.
            // This keeps CPU accounting atomic at the manager decision level.
            pool.book_batch(batch_cost, batch_len);

            for task in batch {
                match task.kind {
                    Kind::Io => dispatched_io += 1,
                    Kind::Cpu => dispatched_cpu += 1,
                }
                worker_tx.send(task).expect("worker channel closed");
            }
            continue;
        }

        if queue.is_closed_and_empty() {
            break;
        }

        // No runnable task right now. Sleep until either:
        //   - generator pushes/closes, or
        //   - worker releases resources.
        seen = signal.wait_for_change(seen);
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
// Note: CPU is *booked* by the manager at dispatch. The worker only releases.

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
// workers, queue length) and writes a CSV row. Stops when the main thread sets
// the shutdown flag.

fn run_monitor(
    cfg: Config,
    queue: Arc<ManagerQueue>,
    pool: Arc<PoolState>,
    shutdown: Arc<AtomicBool>,
    t0: Instant,
) -> (f64, f64, u64) {
    // Returns (avg_cpu_pct, avg_workers_active, samples_taken).
    let mut file = fs::File::create(&cfg.csv_path).expect("cannot create CSV");
    writeln!(file, "elapsed_ms,cpu_pct,workers_active,queue_len").unwrap();

    let mut cpu_sum: u64 = 0;
    let mut workers_sum: u64 = 0;
    let mut samples: u64 = 0;
    let interval = Duration::from_millis(cfg.monitor_interval_ms);

    while !shutdown.load(Ordering::SeqCst) {
        let (cpu, busy) = pool.snapshot();
        let qlen = queue.len();
        let elapsed = t0.elapsed().as_millis();
        writeln!(file, "{},{},{},{}", elapsed, cpu, busy, qlen).unwrap();
        cpu_sum += cpu as u64;
        workers_sum += busy as u64;
        samples += 1;
        thread::sleep(interval);
    }
    let avg_cpu = if samples == 0 {
        0.0
    } else {
        cpu_sum as f64 / samples as f64
    };
    let avg_workers = if samples == 0 {
        0.0
    } else {
        workers_sum as f64 / samples as f64
    };
    (avg_cpu, avg_workers, samples)
}

// ─────────────────────── Main ─────────────────────────────────

fn main() {
    // Load config
    let cfg_text = fs::read_to_string("config.toml").expect("config.toml missing");
    let cfg: Config = toml::from_str(&cfg_text).expect("config.toml is malformed");

    assert!(cfg.io_fraction >= 0.0 && cfg.io_fraction <= 1.0);
    assert!(cfg.num_workers > 0, "num_workers must be > 0");
    assert!(cfg.io_cpu_pct <= cfg.cpu_cap_pct, "IO task cost exceeds CPU cap");
    assert!(cfg.cpu_cpu_pct <= cfg.cpu_cap_pct, "CPU task cost exceeds CPU cap");

    println!("== mode-batch two-queue optimized simulation ==");
    println!(
        "{} tasks, {:.0}% IO / {:.0}% CPU, {} workers, cap {}%",
        cfg.total_tasks,
        cfg.io_fraction * 100.0,
        (1.0 - cfg.io_fraction) * 100.0,
        cfg.num_workers,
        cfg.cpu_cap_pct,
    );

    // Shared state
    let signal = Arc::new(SchedulerSignal::new());
    let queue = Arc::new(ManagerQueue::new(Arc::clone(&signal)));
    let pool = Arc::new(PoolState::new(
        cfg.cpu_cap_pct,
        cfg.num_workers,
        Arc::clone(&signal),
    ));
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
    let monitor_handle = thread::spawn(move || run_monitor(mon_cfg, mon_q, mon_pool, mon_shutdown, t0));

    // Spawn manager
    let mgr_cfg = cfg.clone();
    let mgr_q = Arc::clone(&queue);
    let mgr_pool = Arc::clone(&pool);
    let mgr_signal = Arc::clone(&signal);
    let manager_handle = thread::spawn(move || {
        run_manager(mgr_cfg, mgr_q, mgr_pool, mgr_signal, worker_tx);
    });

    // Spawn generator
    let gen_cfg = cfg.clone();
    let gen_q = Arc::clone(&queue);
    let generator_handle = thread::spawn(move || {
        run_generator(gen_cfg, gen_q);
    });

    // Shutdown sequence — each step waits for the previous to finish:
    //   1. generator stops sending and closes the queues
    //   2. manager drains both queues and drops worker_tx
    //   3. workers see channel closed, exit
    //   4. monitor flag is flipped, monitor exits
    generator_handle.join().unwrap();
    manager_handle.join().unwrap();
    for h in worker_handles {
        h.join().unwrap();
    }
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
        if wait > max_wait {
            max_wait = wait;
        }
        if c.end > makespan_end {
            makespan_end = c.end;
        }
        match c.kind {
            Kind::Io => io_done += 1,
            Kind::Cpu => cpu_done += 1,
        }
    }
    let makespan = makespan_end.duration_since(t0).as_millis();

    println!();
    println!("── results ──");
    println!("total runtime         : {} ms", total_runtime.as_millis());
    println!("makespan              : {} ms", makespan);
    println!(
        "tasks completed       : {}  (IO={}, CPU={})",
        done_count.load(Ordering::SeqCst),
        io_done,
        cpu_done
    );
    println!("avg wait time         : {:.2} ms", wait_sum as f64 / n);
    println!("avg turnaround time   : {:.2} ms", turn_sum as f64 / n);
    println!("max wait time         : {} ms", max_wait);
    println!("avg CPU usage         : {:.2} %", avg_cpu);
    println!(
        "avg workers active    : {:.2} / {}",
        avg_workers, cfg.num_workers
    );
    println!("monitor samples       : {}", samples);
    println!("monitor csv           : {}", cfg.csv_path);
}
