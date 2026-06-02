// Concurrent Task Dispatcher — Two-queue, target-pattern (Simulation 2)
//
// Pipeline:
//   Generator ──tasks──►  Manager ┬──IoQueue──┐
//                                 │           ├──assign──► Worker pool (8)
//                                 └──CpuQueue─┘
//                                 │
//                                 └──► Monitor (samples every 10ms ──► CSV)
//
// The story of how we got here:
//
//   v1: two queues, "try IO first." Worse than FIFO — IO greedily ate
//       slots, CPU queue blew up to 247.
//   v2: "try CPU first." Better, but stuck at 95% CPU usage. At 1 CPU +
//       6 IO, an IO finishes and IO refills before CPU can ever dispatch
//       a second one. Local-optimum trap. CPU peak still 192.
//   v3: added reservation. Inert — the rule only fires at cpu_used ≤ 65,
//       which steady-state never visits. Same numbers as v2.
//   v4 (this version): target-state dispatch. Pick a target running mix
//       from queue contents, then refuse to dispatch a class that's
//       already at target, even if it'd fit. The refusal is what lets
//       the running mix drift toward the right pattern.
//
// Three target patterns saturate the cap at different mixes:
//   pattern (1):  8 IO + 0 CPU  =  80% CPU,  8 workers     (no CPU work)
//   pattern (2):  6 IO + 1 CPU  =  95% CPU,  7 workers     (IO/CPU >= 6:1)
//   pattern (3):  3 IO + 2 CPU  = 100% CPU,  5 workers     (IO/CPU < 6:1)
//
// Targets are chosen from queue contents — they emerge from the policy,
// not enforced by a state machine. See `choose_target` and the manager
// comment for the trap-breaking trace.
//
// Everything else (Task, queues, generator, workers, monitor) keeps the
// same shape as the FIFO baseline so the two simulations stay directly
// comparable.

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
    // else (io then cpu). Used by the monitor for CSV logging and by the
    // manager for target-pattern selection.
    fn lens(&self) -> (usize, usize) {
        let io = self.io.lock().unwrap().len();
        let cpu = self.cpu.lock().unwrap().len();
        (io, cpu)
    }
}

// Counters track the running mix BY CLASS, not just a total. The manager
// needs (io_running, cpu_running) separately to compare the current
// running mix against the target pattern. Total busy = io + cpu, computed
// on demand for the worker-slot check and the monitor.
struct Counters {
    cpu_used: u32,
    io_running: usize,
    cpu_running: usize,
}

impl Counters {
    fn busy_workers(&self) -> usize {
        self.io_running + self.cpu_running
    }
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
            inner: Mutex::new(Counters { cpu_used: 0, io_running: 0, cpu_running: 0 }),
            cv: Condvar::new(),
            cap,
            n_workers,
        }
    }
    // Atomically check the policy AND book the task in the same critical
    // section. Three constraints must all hold:
    //
    //   1. CPU headroom: cpu_used + task_cost <= cap          (physical)
    //   2. Worker slot:  busy_workers < n_workers             (physical)
    //   3. Under target: running_for_class < target_for_class (POLICY)
    //
    // The third constraint is what makes this version different. Previous
    // attempts dispatched whatever fit; this version refuses to dispatch
    // a class that's already at its target count for the current mix.
    // That refusal is what lets the OTHER class catch up — e.g. refusing
    // to dispatch a 7th IO when target is 6 means the slot stays open
    // for the next CPU finish, which then dispatches a CPU and we move
    // toward pattern (3) instead of getting stuck at pattern (2).
    //
    // We also need this to be one atomic critical section because between
    // "check target" and "book", another worker could finish and shift
    // running_for_class — same TOCTOU concern as the original FIFO design,
    // just with one more counter to read.
    fn try_book_against_target(
        &self,
        kind: Kind,
        task_cost: u32,
        target_for_class: usize,
    ) -> bool {
        let mut g = self.inner.lock().unwrap();
        let running_for_class = match kind {
            Kind::Io  => g.io_running,
            Kind::Cpu => g.cpu_running,
        };
        let fits_cpu     = g.cpu_used + task_cost <= self.cap;
        let has_worker   = g.busy_workers() < self.n_workers;
        let under_target = running_for_class < target_for_class;
        if fits_cpu && has_worker && under_target {
            g.cpu_used += task_cost;
            match kind {
                Kind::Io  => g.io_running  += 1,
                Kind::Cpu => g.cpu_running += 1,
            }
            true
        } else {
            false
        }
    }
    // Workers now release by class so we can decrement the right counter.
    fn release(&self, kind: Kind, task_cost: u32) {
        {
            let mut g = self.inner.lock().unwrap();
            g.cpu_used = g.cpu_used.saturating_sub(task_cost);
            match kind {
                Kind::Io  => g.io_running  = g.io_running.saturating_sub(1),
                Kind::Cpu => g.cpu_running = g.cpu_running.saturating_sub(1),
            }
        }
        self.cv.notify_all();
    }
    fn snapshot(&self) -> (u32, usize) {
        let g = self.inner.lock().unwrap();
        (g.cpu_used, g.busy_workers())
    }
    // Manager parks here when nothing is dispatchable. Anyone who changes
    // the situation (worker finishes, generator pushes a task, generator
    // closes the queues) calls notify_all() so the manager wakes up.
    fn wait_for_change(&self) {
        // `let _guard = ...` keeps the guard alive until end of statement,
        // which is what we want — `let _ = ...` would drop it mid-stmt
        // (lint: let_underscore_lock).
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
// The policy in plain English:
//
//   Pick a TARGET running mix based on what's left in the queues, then
//   dispatch tasks to bring the running mix toward that target. Don't
//   exceed the target on either class.
//
//   Three target patterns — each saturates the cap differently:
//     pattern (1):  8 IO + 0 CPU  =  80% CPU,  8 workers
//     pattern (2):  6 IO + 1 CPU  =  95% CPU,  7 workers  (ratio 6:1)
//     pattern (3):  3 IO + 2 CPU  = 100% CPU,  5 workers  (ratio 1.5:1)
//
//   How do we choose the target?
//
//     - If CPU queue is empty:                          target = pattern (1)
//     - Else if io_queue / cpu_queue >= 6:              target = pattern (2)
//     - Else:                                           target = pattern (3)
//
//   The 6:1 boundary is pattern (2)'s consumption ratio. If the queues
//   have IO/CPU >= 6, running pattern (2) drains both at the same rate
//   they're queued. If IO/CPU < 6, CPU is queued faster than pattern (2)
//   drains it, so we need pattern (3) — which drains CPU at 2-in-flight
//   instead of 1-in-flight.
//
//   How does this break the 1+6 trap?
//
//   Old behavior: at 1 CPU + 6 IO running, an IO finishes. Greedy "fits?"
//   says yes (10% room available, 1 slot free) → dispatch IO. Back to
//   1 CPU + 6 IO. Forever.
//
//   New behavior: at 1 CPU + 6 IO with target (3, 2), io_running (6)
//   exceeds target.io (3). Manager refuses to dispatch IO even though
//   it'd fit. So when IO finishes we go to 1 CPU + 5 IO. Still over
//   target on IO. Refuse again. Down to 1 CPU + 4 IO. ... Eventually
//   down to 1 CPU + 3 IO, at which point CPU dispatch kicks in (cpu_running
//   = 1 < target.cpu = 2, headroom 35+30=65, fits 35) and we land at
//   2 CPU + 3 IO = 100%.
//
//   Yes, this means a brief drawdown phase where worker count drops while
//   we wait for IO tasks to finish. That's the price of breaking out of
//   the local optimum. It's worth it because pattern (3) drains the CPU
//   queue twice as fast.

#[derive(Debug, Clone, Copy)]
struct Target {
    io: usize,
    cpu: usize,
}

const PATTERN_1: Target = Target { io: 8, cpu: 0 };
const PATTERN_2: Target = Target { io: 6, cpu: 1 };
const PATTERN_3: Target = Target { io: 3, cpu: 2 };

// Decide target pattern from queue contents. Pure function — no shared
// state, no locks. The arithmetic is deliberately integer-only to avoid
// floating-point comparisons in the hot path; we compare io_q*1 to cpu_q*6
// instead of io_q/cpu_q to 6.
fn choose_target(io_q: usize, cpu_q: usize) -> Target {
    if cpu_q == 0 {
        PATTERN_1
    } else if io_q >= 6 * cpu_q {
        // IO is at least 6x CPU in the queues — IO surplus is large
        // enough that pattern (2) drains them in proportion.
        PATTERN_2
    } else {
        // CPU is queued at least at 1/6 the rate of IO; we need pattern
        // (3) to drain CPU faster, otherwise CPU queue falls behind.
        PATTERN_3
    }
}

fn run_manager(
    queues: Arc<TwoQueues>,
    pool: Arc<PoolState>,
    worker_tx: std::sync::mpsc::Sender<Task>,
) {
    loop {
        if queues.is_closed() && queues.both_empty() {
            break;
        }

        let (io_q, cpu_q) = queues.lens();
        let target = choose_target(io_q, cpu_q);
        let (io_head, cpu_head) = queues.peek_heads();
        let mut dispatched_anything = false;

        // Try CPU first. try_book_against_target enforces three things
        // atomically: physical fit (cap, slot) AND policy (under target).
        // If we're already at or above target.cpu, it refuses regardless
        // of whether the task would physically fit — that's the whole
        // point of target-state dispatch.
        if let Some(cost) = cpu_head {
            if pool.try_book_against_target(Kind::Cpu, cost, target.cpu) {
                let task = queues.pop_cpu().expect("cpu head vanished");
                worker_tx.send(task).expect("worker channel closed");
                dispatched_anything = true;
            }
        }

        // IO second, same rule against target.io. If io_running >= target.io,
        // we refuse — even if a slot is free and there's CPU headroom.
        // This is the trap-breaker: we let workers go idle briefly so the
        // running mix can drift toward pattern (3).
        if let Some(cost) = io_head {
            if pool.try_book_against_target(Kind::Io, cost, target.io) {
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
        pool.release(task.kind, task.cpu_cost); // also notifies the manager

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
    println!("== Optimized simulation ==");
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