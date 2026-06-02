// Concurrent Task Dispatcher — CPU-Leaning Mode-Batch Two-Queue Scheduler (Simulation 4)
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
// Difference from Simulation 3:
//   Simulation 3 tried to keep the cumulative dispatched mix close to the
//   configured global target, for example 70% IO / 30% CPU. This version is
//   intentionally more biased toward the CPU-heavier mode.
//
// Policy idea:
//   The exact-count math for 700 IO / 300 CPU says the optimized recipe is
//       2 rounds  of 8 IO
//       52 rounds of 6 IO + 1 CPU
//       124 rounds of 3 IO + 2 CPU
//   So most rounds should be 3 IO + 2 CPU. If the scheduler overuses IO-heavy
//   rounds early, it can drain IO too aggressively and leave CPU work harder to
//   pair later. This manager therefore prefers 3 IO + 2 CPU by default.
//
// Dynamic rule:
//   - If no CPU tasks are queued, use 8 IO.
//   - Else if the queued IO/CPU ratio is at least 6/1, use 6 IO + 1 CPU.
//   - Otherwise prefer 3 IO + 2 CPU.
//
// Important simulation caveat:
//   The exact recipe assumes all 700/300 tasks are already waiting. This program
//   has arrivals over time, CPU caps, and worker limits. So the recipe becomes
//   a dynamic policy: try the preferred full mode first, then use a smaller
//   fallback batch when arrivals/resources make a full mode impossible.
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
    // This Simulation 4 policy is deliberately NOT the same as Simulation 3.
    // Simulation 3 asked: "Which full mode makes my cumulative dispatched ratio
    // closest to target?" That is mathematically tidy, but it can become too
    // balanced too late.
    //
    // Here we use a more practical queue rule:
    //   1. If no CPU work is waiting, try 8 IO.
    //   2. If IO backlog is at least 6x CPU backlog, try 6 IO + 1 CPU.
    //   3. Otherwise, prefer 3 IO + 2 CPU.
    //
    // That means 3 IO + 2 CPU becomes the normal/default mode. The 6 IO + 1 CPU
    // mode is reserved for moments when the IO queue is clearly piling up.
    //
    // This method does NOT wait. Waiting is handled by run_manager using
    // SchedulerSignal, because the manager may need to wake up for either a
    // task-arrival event or a resource-release event.
    fn pop_prefer_3io2cpu_batch_that_fits(
        &self,
        cpu_room: u32,
        free_workers: usize,
        io_cost: u32,
        cpu_cost: u32,
        target_io_fraction: f64,
    ) -> Option<Vec<Task>> {
        if free_workers == 0 {
            return None;
        }

        let mut q = self.inner.lock().unwrap();

        // ── 1) Choose the preferred full-mode order ─────────────
        // The preferred mode is chosen from the live queue shape, not from a
        // global cumulative ratio. This is the behavior requested for this
        // version: lean into 3 IO + 2 CPU unless IO is extremely dominant.
        let cpu_waiting = q.cpu.len();
        let io_waiting = q.io.len();

        let preferred_modes: [DispatchMode; 3] = if cpu_waiting == 0 {
            [
                DispatchMode { name: "8 IO", io: 8, cpu: 0 },
                DispatchMode { name: "6 IO + 1 CPU", io: 6, cpu: 1 },
                DispatchMode { name: "3 IO + 2 CPU", io: 3, cpu: 2 },
            ]
        } else if io_waiting >= 6 * cpu_waiting {
            [
                DispatchMode { name: "6 IO + 1 CPU", io: 6, cpu: 1 },
                DispatchMode { name: "3 IO + 2 CPU", io: 3, cpu: 2 },
                DispatchMode { name: "8 IO", io: 8, cpu: 0 },
            ]
        } else {
            [
                DispatchMode { name: "3 IO + 2 CPU", io: 3, cpu: 2 },
                DispatchMode { name: "6 IO + 1 CPU", io: 6, cpu: 1 },
                DispatchMode { name: "8 IO", io: 8, cpu: 0 },
            ]
        };

        // ── 2) Try the full modes in that order ─────────────────
        for mode in preferred_modes {
            let enough_tasks = q.io.len() >= mode.io && q.cpu.len() >= mode.cpu;
            let enough_workers = mode.total() <= free_workers;
            let enough_cpu = mode.cost(io_cost, cpu_cost) <= cpu_room;

            if enough_tasks && enough_workers && enough_cpu {
                let mut batch = Vec::with_capacity(mode.total());
                Self::pop_n(&mut q.io, mode.io, &mut batch);
                Self::pop_n(&mut q.cpu, mode.cpu, &mut batch);
                return Some(batch);
            }
        }

        // ── 3) Fallback partial batch ───────────────────────────
        // Full batches are clean for the math explanation, but arrivals are
        // streaming. A strict full-mode-only manager can leave workers idle
        // while waiting for a perfect 5/7/8-task bundle. This fallback still
        // follows the same bias: CPU is protected unless the IO queue is very
        // dominant or CPU is unavailable.
        let mut batch = Vec::new();
        let mut remaining_workers = free_workers;
        let mut remaining_cpu_room = cpu_room;
        let mut local_io = 0usize;
        let mut local_cpu = 0usize;

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
                    let cpu_waiting = q.cpu.len();
                    let io_waiting = q.io.len();

                    if cpu_waiting == 0 {
                        Some(Kind::Io)
                    } else if io_waiting >= 6 * cpu_waiting {
                        // IO is extremely dominant: approximate 6 IO : 1 CPU.
                        if local_io < 6 * (local_cpu + 1) {
                            Some(Kind::Io)
                        } else {
                            Some(Kind::Cpu)
                        }
                    } else {
                        // Normal/default behavior: approximate 3 IO : 2 CPU.
                        // Compare where the next pick would move the partial
                        // batch relative to the 3/2 target.
                        let err_if_io = ratio_error(local_io + 1, local_cpu, 3.0 / 5.0);
                        let err_if_cpu = ratio_error(local_io, local_cpu + 1, 3.0 / 5.0);

                        // Small extra guard: if this run is configured for a
                        // very IO-heavy workload, do not force CPU beyond what
                        // the configuration says forever. But for the common
                        // 70/30 case, this still strongly leans into 3/2 early.
                        let configured_cpu_fraction = 1.0 - target_io_fraction.clamp(0.0, 1.0);
                        let partial_total = local_io + local_cpu;
                        let partial_cpu_fraction = if partial_total == 0 {
                            0.0
                        } else {
                            local_cpu as f64 / partial_total as f64
                        };

                        if partial_total >= 3 && partial_cpu_fraction > configured_cpu_fraction + 0.20 {
                            Some(Kind::Io)
                        } else if err_if_cpu <= err_if_io {
                            Some(Kind::Cpu)
                        } else {
                            Some(Kind::Io)
                        }
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
                Kind::Io => local_io += 1,
                Kind::Cpu => local_cpu += 1,
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
//   - It prefers 3 IO + 2 CPU unless the IO backlog is roughly 6x CPU.

fn run_manager(
    cfg: Config,
    queue: Arc<ManagerQueue>,
    pool: Arc<PoolState>,
    signal: Arc<SchedulerSignal>,
    worker_tx: std::sync::mpsc::Sender<Task>,
) {
    let mut seen = signal.version();
    loop {
        let (cpu_room, free_workers) = pool.available();

        if let Some(batch) = queue.pop_prefer_3io2cpu_batch_that_fits(
            cpu_room,
            free_workers,
            cfg.io_cpu_pct,
            cfg.cpu_cpu_pct,
            cfg.io_fraction,
        ) {
            let batch_cost = batch.iter().map(|task| task.cpu_cost).sum::<u32>();
            let batch_len = batch.len();

            // Book all resources for the batch before any worker receives a task.
            // This keeps CPU accounting atomic at the manager decision level.
            pool.book_batch(batch_cost, batch_len);

            for task in batch {
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

    println!("== prefer-3io2cpu two-queue optimized simulation ==");
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
