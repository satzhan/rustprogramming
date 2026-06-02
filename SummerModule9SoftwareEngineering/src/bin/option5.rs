// Concurrent Task Dispatcher — Two-queue optimized (Simulation 2)
//
// Pipeline:
//   Generator ──tasks──►  Manager ┬──IoQueue──┐
//                                 │           ├──assign──► Worker pool (8)
//                                 └──CpuQueue─┘
//                                 │
//                                 └──► Monitor (samples every 10ms ──► CSV)
//
// What changed from FIFO (option1.rs) and why:
//
//   FIFO has one queue. If the head is a CPU task and CPU is saturated, the
//   manager waits — even when 5 worker slots are idle and the next 40 tasks
//   in the queue are tiny IO tasks that would fit in 10% CPU each. That's
//   head-of-line blocking, and on a 70/30 IO-heavy workload it leaves a lot
//   of throughput on the floor.
//
//   The fix is structural, not algorithmic: separate the tasks by the
//   resource they're constrained on. IO tasks compete for worker slots
//   (cheap on CPU). CPU tasks compete for CPU headroom (cheap on slots).
//   With two queues, an IO task in lane A doesn't have to wait for a
//   stuck CPU task in lane B. They use different bottlenecks.
//
//   Within each lane, dispatch is still FIFO — so we keep per-class
//   ordering. Across lanes, CPU has priority: whenever a CPU task fits,
//   dispatch it before considering IO. CPU is the bottleneck (max 2 in
//   flight at the cap), so draining its queue first is what keeps the
//   CPU queue from blowing up. IO doesn't starve because IO tasks are
//   only 10% CPU each — even with 2 CPU tasks running (70%), there's
//   still room for 3 IO tasks plus 5 free workers.
//
//   Everything else (Task, PoolState, workers, monitor) is unchanged so
//   the two simulations are directly comparable.

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

// ─────────────────────── Shared state ─────────────────────────
//
// Three pieces of mutable shared state, each behind its own lock:
//
//   1. TwoQueues  — IO FIFO + CPU FIFO (both protected by one mutex so the
//                   manager can inspect both heads in a single critical
//                   section before deciding what to dispatch)
//   2. PoolState  — global CPU% + busy worker count (same as FIFO version)
//
// The PoolState condvar is the manager's "something changed, re-check"
// signal. It fires when:
//   - a worker finishes (resources freed)        ← same as FIFO
//   - a new task is pushed into either queue     ← NEW: needed because the
//                                                   manager may be parked
//                                                   on this condvar with
//                                                   empty queues, and we
//                                                   need to wake it when
//                                                   work arrives.
//   - the queues are closed (shutdown)
//
// One condvar handles all three because the manager's reaction is the
// same in every case: re-check whether anything is now dispatchable.

struct TwoQueues {
    io: Mutex<VecDeque<Task>>,
    cpu: Mutex<VecDeque<Task>>,
    closed: AtomicBool,
}

impl TwoQueues {
    fn new() -> Self {
        Self {
            io: Mutex::new(VecDeque::new()),
            cpu: Mutex::new(VecDeque::new()),
            closed: AtomicBool::new(false),
        }
    }
    // Push routes by kind. Notify happens via PoolState — see push_and_notify.
    fn push(&self, t: Task) {
        match t.kind {
            Kind::Io => self.io.lock().unwrap().push_back(t),
            Kind::Cpu => self.cpu.lock().unwrap().push_back(t),
        }
    }
    fn close(&self) {
        self.closed.store(true, Ordering::SeqCst);
    }
    fn is_closed(&self) -> bool {
        self.closed.load(Ordering::SeqCst)
    }
    // Peek the head cost of each queue without removing. Returns
    // (io_head_cost, cpu_head_cost). None means that queue is empty.
    // Locks are taken in a fixed order (io then cpu) — same order
    // everywhere, so no deadlock risk.
    fn peek_heads(&self) -> (Option<u32>, Option<u32>) {
        let io = self.io.lock().unwrap();
        let cpu = self.cpu.lock().unwrap();
        (
            io.front().map(|t| t.cpu_cost),
            cpu.front().map(|t| t.cpu_cost),
        )
    }
    fn pop_io(&self) -> Option<Task> {
        self.io.lock().unwrap().pop_front()
    }
    fn pop_cpu(&self) -> Option<Task> {
        self.cpu.lock().unwrap().pop_front()
    }
    fn both_empty(&self) -> bool {
        self.io.lock().unwrap().is_empty() && self.cpu.lock().unwrap().is_empty()
    }
    // One snapshot, two locks taken in the same fixed order as everywhere
    // else (io then cpu). Avoids two separate critical sections in the
    // monitor's hot path.
    fn lens(&self) -> (usize, usize) {
        let io = self.io.lock().unwrap().len();
        let cpu = self.cpu.lock().unwrap().len();
        (io, cpu)
    }
}

// PoolState is identical to the FIFO version — same TOCTOU reasoning,
// same atomic book/release. We just call book_for from a smarter manager.
struct Counters {
    cpu_used: u32,
    busy_workers: usize,
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
            inner: Mutex::new(Counters { cpu_used: 0, busy_workers: 0 }),
            cv: Condvar::new(),
            cap,
            n_workers,
        }
    }
    // Try to atomically book resources for a task of the given cost.
    // Returns true on success, false if it doesn't fit right now.
    // Non-blocking — the manager is the one that decides whether/when
    // to retry, because it may want to try the OTHER queue first.
    fn try_book(&self, task_cost: u32) -> bool {
        let mut g = self.inner.lock().unwrap();
        let fits_cpu = g.cpu_used + task_cost <= self.cap;
        let has_worker = g.busy_workers < self.n_workers;
        if fits_cpu && has_worker {
            g.cpu_used += task_cost;
            g.busy_workers += 1;
            true
        } else {
            false
        }
    }
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
    // Manager parks here when nothing is dispatchable. Anyone who changes
    // the situation (worker finishes, generator pushes a task, generator
    // closes the queues) calls notify_all() so the manager wakes up and
    // re-evaluates.
    fn wait_for_change(&self) {
        // Park on the condvar until something changes (worker finishes,
        // task pushed, or queues closed). We deliberately don't bind the
        // returned guard — we don't need to read or modify the counters
        // here, we just needed to sleep. The guard is dropped at the
        // end of this statement, immediately after wait() returns.
        //
        // NB: `let _ = self.cv.wait(g)` would be a bug — the underscore
        // pattern drops the guard mid-statement, which is why rustc lints
        // it as `let_underscore_lock`. Letting the temporary drop at
        // statement-end is the correct shape.
        let g = self.inner.lock().unwrap();
        let _guard = self.cv.wait(g).unwrap();
    }
    fn notify(&self) {
        self.cv.notify_all();
    }
}

// ─────────────────────── Generator ────────────────────────────
//
// Same as FIFO version, except: after pushing a task we notify the pool
// condvar so a sleeping manager wakes up. (FIFO didn't need this because
// its queue had its own condvar; here we centralize all "wake the manager"
// signals on one condvar to keep the manager loop simple.)

fn run_generator(cfg: Config, queues: Arc<TwoQueues>, pool: Arc<PoolState>) {
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
        queues.push(task);
        pool.notify(); // wake manager if it's parked on an empty pair of queues
        thread::sleep(interval);
    }
    queues.close();
    pool.notify(); // wake manager so it sees `closed && both_empty()` and exits
}

// ─────────────────────── Manager ──────────────────────────────
//
// One thread. Each pass:
//   1. Peek both queue heads.
//   2. Try to book the CPU head (35% CPU). If it fits, dispatch it.
//      → CPU tasks make progress whenever there's CPU headroom.
//   3. Try to book the IO head (cheap, 10% CPU). If it fits, dispatch it.
//      → IO tasks fill remaining slots whenever a worker is free.
//   4. If neither fit (or both queues empty but generator still running),
//      park on the pool condvar until a worker finishes or a task arrives.
//   5. Exit when generator has closed the queues AND both queues are empty.
//
// The rule is simply: CPU has priority. Why? CPU tasks are the bottleneck.
// At 100% cap with 35%/CPU and 10%/IO, you can run at most 2 CPU tasks
// concurrently — so CPU throughput maxes at ~2 in flight, while IO maxes
// at 8 (worker-bound). The queue that fills up is the one with the lower
// drain rate, and that's CPU. If we let IO tasks dispatch first, they
// happily eat worker slots and drain a queue that wasn't going to be a
// problem anyway, while CPU tasks pile up.
//
// CPU-first inverts that: we drain the slow queue when we can. IO tasks
// are 10% CPU each, so even with 2 CPU tasks running (70% used), we still
// have 30% CPU headroom — enough for 3 IO tasks in parallel — and 5 free
// worker slots. So IO doesn't starve; it just stops jumping the line.
//
// This produces three natural running mixes that saturate the cap:
//   * 8 IO         (80% CPU)  — when no CPU tasks are queued
//   * 1 CPU + 6 IO (95% CPU)  — middle ground
//   * 2 CPU + 3 IO (100% CPU) — both bottlenecks engaged
// We don't enforce these directly; they emerge from the priority rule.

fn run_manager(
    queues: Arc<TwoQueues>,
    pool: Arc<PoolState>,
    worker_tx: std::sync::mpsc::Sender<Task>,
) {
    loop {
        if queues.is_closed() && queues.both_empty() {
            break;
        }

        let (io_head, cpu_head) = queues.peek_heads();
        let mut dispatched_anything = false;

        // CPU lane FIRST. The manager is the only popper, so peek-then-pop
        // is race-free even though it's not atomic.
        if let Some(cost) = cpu_head {
            if pool.try_book(cost) {
                let task = queues.pop_cpu().expect("cpu head vanished");
                worker_tx.send(task).expect("worker channel closed");
                dispatched_anything = true;
            }
        }

        // IO lane second. We try this even if we just dispatched a CPU
        // task — there might still be CPU headroom AND a free slot for
        // an IO task. (e.g. 35% used after one CPU dispatch → still 65%
        // headroom for 6 IO tasks, plus more CPU room.)
        if let Some(cost) = io_head {
            if pool.try_book(cost) {
                let task = queues.pop_io().expect("io head vanished");
                worker_tx.send(task).expect("worker channel closed");
                dispatched_anything = true;
            }
        }

        if !dispatched_anything {
            pool.wait_for_change();
        }
    }
    drop(worker_tx);
}

// ─────────────────────── Worker pool ──────────────────────────
//
// Identical to FIFO version. Workers don't know or care that there are
// two queues — by the time a task arrives on the channel, the manager
// has already decided everything.

type SharedRx = Arc<Mutex<std::sync::mpsc::Receiver<Task>>>;

fn run_worker(
    _id: usize,
    rx: SharedRx,
    pool: Arc<PoolState>,
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
        pool.release(task.cpu_cost); // this also notifies the manager

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

// ─────────────────────── Monitor ──────────────────────────────
//
// Same shape as FIFO version. CSV now records three queue columns
// (queue_io, queue_cpu, queue_total) instead of one — this is the
// metric that actually shows whether the two-queue design is helping
// or hurting. If one lane stays full while the other drains, that's
// a starvation signal we can act on.

fn run_monitor(
    cfg: Config,
    queues: Arc<TwoQueues>,
    pool: Arc<PoolState>,
    shutdown: Arc<AtomicBool>,
    t0: Instant,
) -> (f64, f64, u64) {
    let mut file = fs::File::create(&cfg.csv_path).expect("cannot create CSV");
    writeln!(file, "elapsed_ms,cpu_pct,workers_active,queue_io,queue_cpu,queue_total").unwrap();

    let mut cpu_sum: u64 = 0;
    let mut workers_sum: u64 = 0;
    let mut samples: u64 = 0;
    let interval = Duration::from_millis(cfg.monitor_interval_ms);

    while !shutdown.load(Ordering::SeqCst) {
        let (cpu, busy) = pool.snapshot();
        let (q_io, q_cpu) = queues.lens();
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
    let cfg_text = fs::read_to_string("config.toml").expect("config.toml missing");
    let cfg: Config = toml::from_str(&cfg_text).expect("config.toml is malformed");
    println!("== Two-queue (optimized) simulation ==");
    println!("{} tasks, {:.0}% IO / {:.0}% CPU, {} workers, cap {}%",
        cfg.total_tasks,
        cfg.io_fraction * 100.0,
        (1.0 - cfg.io_fraction) * 100.0,
        cfg.num_workers,
        cfg.cpu_cap_pct,
    );

    let queues = Arc::new(TwoQueues::new());
    let pool = Arc::new(PoolState::new(cfg.cpu_cap_pct, cfg.num_workers));
    let completions = Arc::new(Mutex::new(Vec::with_capacity(cfg.total_tasks)));
    let done_count = Arc::new(AtomicUsize::new(0));
    let shutdown = Arc::new(AtomicBool::new(false));

    let (worker_tx, worker_rx) = std::sync::mpsc::channel::<Task>();
    let worker_rx: SharedRx = Arc::new(Mutex::new(worker_rx));

    let t0 = Instant::now();

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

    let mon_cfg = cfg.clone();
    let mon_q = Arc::clone(&queues);
    let mon_pool = Arc::clone(&pool);
    let mon_shutdown = Arc::clone(&shutdown);
    let monitor_handle = thread::spawn(move || {
        run_monitor(mon_cfg, mon_q, mon_pool, mon_shutdown, t0)
    });

    let mgr_q = Arc::clone(&queues);
    let mgr_pool = Arc::clone(&pool);
    let manager_handle = thread::spawn(move || {
        run_manager(mgr_q, mgr_pool, worker_tx);
    });

    let gen_cfg = cfg.clone();
    let gen_q = Arc::clone(&queues);
    let gen_pool = Arc::clone(&pool);
    let generator_handle = thread::spawn(move || {
        run_generator(gen_cfg, gen_q, gen_pool);
    });

    // Shutdown sequence is the same shape as FIFO:
    //   1. generator stops, closes queues, notifies once more
    //   2. manager sees closed && both_empty, drops worker_tx
    //   3. workers see channel closed, exit
    //   4. monitor flag flipped, monitor exits
    generator_handle.join().unwrap();
    manager_handle.join().unwrap();
    for h in worker_handles { h.join().unwrap(); }
    shutdown.store(true, Ordering::SeqCst);
    let (avg_cpu, avg_workers, samples) = monitor_handle.join().unwrap();

    let total_runtime = t0.elapsed();

    let comps = completions.lock().unwrap();
    let n = comps.len() as f64;
    let (mut wait_sum, mut turn_sum, mut max_wait) = (0u128, 0u128, 0u128);
    let mut max_wait_id: u64 = 0;
    let (mut io_done, mut cpu_done) = (0u64, 0u64);
    let (mut io_wait_sum, mut cpu_wait_sum) = (0u128, 0u128);
    let mut makespan_end = t0;
    for c in comps.iter() {
        let wait = c.start.duration_since(c.arrival).as_millis();
        let turn = c.end.duration_since(c.arrival).as_millis();
        wait_sum += wait;
        turn_sum += turn;
        if wait > max_wait {
            max_wait = wait;
            max_wait_id = c.id;
        }
        if c.end > makespan_end { makespan_end = c.end; }
        match c.kind {
            Kind::Io  => { io_done  += 1; io_wait_sum  += wait; }
            Kind::Cpu => { cpu_done += 1; cpu_wait_sum += wait; }
        }
    }
    let makespan = makespan_end.duration_since(t0).as_millis();

    // Per-class wait is interesting here — it's the metric that actually
    // shows the head-of-line-blocking story. In FIFO, IO tasks pile up
    // behind stuck CPU tasks; in this version, they shouldn't.
    let avg_io_wait = if io_done > 0 { io_wait_sum as f64 / io_done as f64 } else { 0.0 };
    let avg_cpu_wait = if cpu_done > 0 { cpu_wait_sum as f64 / cpu_done as f64 } else { 0.0 };

    println!();
    println!("── results ──");
    println!("total runtime         : {} ms", total_runtime.as_millis());
    println!("makespan              : {} ms", makespan);
    println!("tasks completed       : {}  (IO={}, CPU={})",
        done_count.load(Ordering::SeqCst), io_done, cpu_done);
    println!("avg wait time         : {:.2} ms", wait_sum as f64 / n);
    println!("avg wait (IO only)    : {:.2} ms", avg_io_wait);
    println!("avg wait (CPU only)   : {:.2} ms", avg_cpu_wait);
    println!("avg turnaround time   : {:.2} ms", turn_sum as f64 / n);
    println!("max wait time         : {} ms (task #{})", max_wait, max_wait_id);
    println!("avg CPU usage         : {:.2} %", avg_cpu);
    println!("avg workers active    : {:.2} / {}", avg_workers, cfg.num_workers);
    println!("monitor samples       : {}", samples);
    println!("monitor csv           : {}", cfg.csv_path);
}