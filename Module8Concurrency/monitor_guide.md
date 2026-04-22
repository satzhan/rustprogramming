# The Monitor — A Ground-Up Guide

*For the Concurrent Task Dispatcher Final Project (Rust)*

---

## Part 1 — What Is a Monitor?

### The one-sentence definition

A **monitor** is a thread whose only job is to *watch* what the rest of the system is doing and *write down* what it sees.

It does not generate tasks. It does not run tasks. It does not dispatch. It *observes*.

That's the whole idea. Everything else in this guide is just us getting comfortable with what "watch" and "write down" actually look like in code.

### Everyday monitors (no computers involved)

Before we touch Rust, let's ground this. Monitors are already everywhere:

- **The speedometer in your car.** The engine is busy doing engine things. The speedometer has one job: look at how fast the wheels are turning and put a number on a dial. It doesn't steer. It doesn't brake. It reports.
- **A heart-rate monitor on a treadmill.** You run. It watches your pulse. Every second or so, it updates a number. If you stop running, the monitor keeps working — it just shows a lower number.
- **A lifeguard in the chair.** The swimmers are the workers. The lifeguard is not swimming. The lifeguard's job is to *sample* the pool every few seconds and note: how many people are in the water, is anyone struggling, is the deep end crowded.
- **The fuel gauge.** It doesn't control the engine. It just peeks at the tank now and then and reports what it saw.

Notice what all of these have in common:

1. The monitor runs on its **own schedule** (tick, tick, tick).
2. The monitor reads things it **doesn't own** (the engine, the pool, the tank).
3. The monitor's output is **a record** — not a control action.
4. If the monitor dies, the main system keeps going. If the main system dies, the monitor eventually has nothing to report.


### Why our final project needs a monitor

Our dispatcher is going to spawn workers. Those workers are going to run tasks. Tasks take time. Some use CPU. Some are I/O-heavy. The manager is deciding what to send where.

Here is the uncomfortable truth: **once the system is running, you cannot just `println!` your way to understanding it.**

If you try to have the workers print their own status, you'll get interleaved, out-of-order, context-free garbage. And you'll slow the workers down by making them do a second job (reporting) on top of their first job (working).

A monitor fixes this by:

- **Separating concerns.** Workers work. The monitor reports. Each thread has one responsibility — which is exactly the principle the handout says to follow ("each part has one job").
- **Giving you time-series data.** The monitor samples every 10 ms. That means at the end of the run, you don't just have "it finished" — you have a *timeline* of CPU usage, worker activity, and completions. That timeline is what you'll use to compare your FIFO run against your optimized run.
- **Making the system explainable during the demo.** When your instructor asks "what was the average CPU usage?" or "were workers idle?", the monitor already has the answer. No guessing.


---

## Part 2 — The Progression (Twelve Tiny Steps)

We're going to build up to the full monitor in small pieces. Each step adds exactly one new idea. If you skip ahead and something confuses you, come back to the step before — the missing concept is almost always there.

The rhythm: **show the smallest version, point at what's new, explain why it matters.**

### Step 1 — A thread that just ticks (`monitor2.rs`)

```rust
let start = Instant::now();

let monitor = thread::spawn(move || {
    for _ in 0..5 {
        let elapsed = start.elapsed().as_millis();
        println!("[monitor] tick at {}ms", elapsed);
        thread::sleep(Duration::from_millis(10));
    }
});

thread::sleep(Duration::from_millis(60));
monitor.join().unwrap();
```

**What this shows.** A thread can run in parallel with main and print on its own heartbeat. `Instant::now()` gives us an anchor point; `elapsed()` tells us how far we've traveled from it. `thread::sleep(10ms)` is the rhythm. `join()` is how main waits for the monitor before exiting.

**What's new:** the monitor exists.

**Analogy.** This is the metronome, sitting on top of the piano, clicking. It doesn't know any music yet. It just clicks.

---

### Step 2 — Watching something shared (`monitor3.rs`)

```rust
let completed = Arc::new(AtomicUsize::new(0));

let worker_counter = Arc::clone(&completed);
let worker = thread::spawn(move || {
    for _ in 0..8 {
        thread::sleep(Duration::from_millis(15));
        worker_counter.fetch_add(1, Ordering::Relaxed);
    }
});

let monitor_counter = Arc::clone(&completed);
let monitor = thread::spawn(move || {
    for _ in 0..15 {
        let n = monitor_counter.load(Ordering::Relaxed);
        // ...print...
        thread::sleep(Duration::from_millis(10));
    }
});
```

**What's new.** A second thread — the "worker" — is now doing real work (well, fake real work). The monitor isn't just ticking in a vacuum; it's *observing something*.

**The two new tools.**

- `Arc<T>` — "Atomically Reference-Counted." Think of it as a shared pointer with a built-in headcount. Every thread that holds one is counted. When the last one drops, the value inside gets freed. It's how we let two threads see the same piece of data without one of them owning it.
- `AtomicUsize` — a counter that can be incremented safely from many threads at once. No lock needed, because the hardware itself guarantees the `fetch_add` is indivisible.

**Why not `Mutex<usize>`?** You could. But a counter is the classic case where an atomic is cheaper and simpler — there's nothing to "lock around" because there's no compound operation. Just "add one." Save the `Mutex` for when you have a `HashMap` or something with structure.

**Analogy.** The speedometer doesn't ask the engine for permission. It reads a signal that the engine is constantly publishing. That's what `load(Relaxed)` is — a quick peek at a value the worker is constantly publishing via `fetch_add`.

---

### Step 3 — Clean shutdown (`monitor4.rs`)

```rust
let shutdown = Arc::new(AtomicBool::new(false));

// monitor loop:
while !monitor_shutdown.load(Ordering::Relaxed) {
    // ...sample and print...
    thread::sleep(Duration::from_millis(10));
}

// later in main:
worker.join().unwrap();
shutdown.store(true, Ordering::Relaxed);
monitor.join().unwrap();
```

**What's new.** Instead of the monitor running for a fixed number of iterations, it runs *until told to stop*. This matches how a real system behaves: we don't know in advance how long the work will take.

**The pattern.** This is the canonical shutdown handshake:

1. Main waits for all *producers of data* (the workers) to finish.
2. Main flips the shutdown flag.
3. Main waits for the *observer of data* (the monitor) to notice and exit.

Order matters. If you flip `shutdown` before the workers finish, the monitor exits early and misses the final bursts. If you forget to flip it at all, `monitor.join()` hangs forever — and the handout explicitly lists "no hanging program at the end" as a hard requirement.

**Analogy.** The lifeguard only climbs down when the pool manager says "we're closed, swimmers are out." Not before.

---

### Step 4 — A second signal to watch (`monitor5.rs`)

```rust
let cpu_usage = Arc::new(AtomicUsize::new(0));

// worker:
worker_cpu.fetch_add(cost, Ordering::Relaxed);   // task starts
thread::sleep(Duration::from_millis(30));         // task runs
worker_cpu.fetch_sub(cost, Ordering::Relaxed);   // task ends
worker_counter.fetch_add(1, Ordering::Relaxed);
```

**What's new.** A second atomic. Now we're tracking *two* things: completions *and* current CPU usage.

**The add/sub pattern.** This is the key idea and it will repeat. When a task starts, we `fetch_add` its cost. When it ends, we `fetch_sub` the same cost. At any moment, `cpu_usage` equals the sum of costs of all tasks currently running. The monitor can peek at this number whenever it wakes up.

**Why this is genuinely parallel-safe.** Two workers can both do `fetch_add(35)` at the exact same nanosecond and the counter will correctly become 70, not 35. That's the guarantee the hardware gives us. No locks, no lost updates.

**Subtle point.** The CPU usage number the monitor reads is always a snapshot of *a specific instant*. It's not an average. It's "right now, at this tick, how much CPU is being consumed." Averaging is the monitor's job later.

---

### Step 5 — Multiple workers (`monitor6.rs`)

```rust
for worker_id in 0..4 {
    let worker_counter = Arc::clone(&completed);
    let worker_cpu = Arc::clone(&cpu_usage);
    let handle = thread::spawn(move || {
        // same loop as before
    });
    worker_handles.push(handle);
}

for handle in worker_handles {
    handle.join().unwrap();
}
```

**What's new.** We're now running four workers in parallel. All four increment the same `completed` counter. All four add to and subtract from the same `cpu_usage` counter.

**Why nothing breaks.** Because atomics. We've already paid for the right to do this when we chose `AtomicUsize` back in Step 2. Had we used a plain `usize`, this would be undefined behavior.

**The `Arc::clone` dance.** Notice we clone the `Arc` *once per worker, before `spawn`*. Each worker closure takes ownership of its own clone. The reference count is now 5 (four workers + main). When workers finish, they drop their clones; the count ticks down. When main drops its last reference, the atomic's memory is freed.

**Analogy.** The speedometer doesn't care if the engine has four cylinders or eight. Each cylinder contributes. The monitor sees the sum.

---

### Step 6 — Counting who's busy (`monitor7.rs`)

```rust
let active_workers = Arc::new(AtomicUsize::new(0));

// worker task:
worker_active.fetch_add(1, Ordering::Relaxed);   // "I'm busy"
worker_cpu.fetch_add(cost, Ordering::Relaxed);
thread::sleep(Duration::from_millis(30));
worker_cpu.fetch_sub(cost, Ordering::Relaxed);
worker_active.fetch_sub(1, Ordering::Relaxed);   // "I'm free"
```

**What's new.** A third atomic — `active_workers` — which tracks how many workers are currently inside a task.

**Why this matters.** CPU usage and worker count answer different questions:

- `cpu_usage` answers: *"Are we running out of CPU headroom?"*
- `active_workers` answers: *"Are there idle workers sitting around doing nothing?"*

Two IO tasks (10% each) on two workers gives `cpu=20%, active=2/4`. One CPU task (35%) alone gives `cpu=35%, active=1/4`. Same system, very different stories. You can't recover one number from the other.

**Analogy.** The lifeguard counts swimmers in the pool, not laps swum. Both are legitimate measurements; they just describe different facets of "how busy is the pool."

---

### Step 7 — Total runtime (`monitor8.rs`)

```rust
for handle in worker_handles {
    handle.join().unwrap();
}

let total_time = start.elapsed();   // ← capture BEFORE shutting down monitor

shutdown.store(true, Ordering::Relaxed);
monitor.join().unwrap();
```

**What's new.** We record `total_time` at the exact moment the last worker finishes — not later, after the monitor has been joined.

**Why the ordering is surgical.** If you measure `total_time` *after* `monitor.join()`, you've included the time the monitor spent shutting down (up to one `monitor_tick_ms` of sleep). For a 10 ms tick on a 300 ms run, that's a 3% error. For comparing FIFO against your optimized run, that's exactly the kind of contamination you want to avoid.

**The rule.** *Stop the clock when the work is done, not when the reporter leaves the room.*

---

### Step 8 — Accumulating for a final report (`monitor9.rs`)

```rust
struct MonitorReport {
    sample_count: u64,
    cpu_sum: u64,
    active_sum: u64,
}

let (report_tx, report_rx) = mpsc::channel::<MonitorReport>();

// monitor keeps LOCAL state:
let mut sample_count: u64 = 0;
let mut cpu_sum: u64 = 0;
let mut active_sum: u64 = 0;

while !monitor_shutdown.load(Ordering::Relaxed) {
    let cpu = monitor_cpu.load(Ordering::Relaxed);
    let active = monitor_active.load(Ordering::Relaxed);
    sample_count += 1;
    cpu_sum += cpu as u64;
    active_sum += active as u64;
    thread::sleep(Duration::from_millis(10));
}

report_tx.send(MonitorReport { sample_count, cpu_sum, active_sum }).unwrap();
```

**What's new.** Two things, and both are deep.

**(a) Local state.** The monitor now keeps its running totals in ordinary local variables — plain `u64`s, no `Arc`, no atomic. Why? Because no one else needs to read them. A thread's private data is cheap and simple. Reach for shared primitives only when you actually have sharing.

**(b) Channels for handoff.** When the monitor is done accumulating, it sends one message — the final report — back to main through an `mpsc` channel.

This is the handout's "channels vs shared state" question in miniature:

- While sampling: we use **shared atomics**, because every tick we're reading something the workers are constantly writing.
- To deliver the summary: we use a **channel**, because it's a one-time handoff from one owner (monitor) to another (main). Ownership transfer is exactly what channels are for.

**Computing averages.** `cpu_sum / sample_count` as a `u64` gives you integer division — `49.7%` silently becomes `49`. Cast to `f64` first. The `{:.1}` in the `println!` is how you print one decimal place.

**Analogy.** The lifeguard keeps a tally sheet on their clipboard all day. Nobody else needs to see the tally sheet during the shift. At the end, the lifeguard hands the manager *one* summary slip. That's the channel.

---

### Step 9 — Real task mix via RNG (`monitor10.rs`)

```rust
enum TaskKind { Cpu, Io }

impl TaskKind {
    fn cpu_cost(&self) -> usize {
        match self { TaskKind::Cpu => 35, TaskKind::Io => 10 }
    }
}

let mut rng = StdRng::seed_from_u64(42 + worker_id as u64);

let kind = if rng.random_bool(0.5) { TaskKind::Io } else { TaskKind::Cpu };
let duration_ms = rng.random_range(20..=40);
```

**What's new.** Tasks now have a *type* and variable durations, picked randomly but *reproducibly*.

**Why seeded RNG.** The handout explicitly calls for a fixed seed. If your experiment can't be re-run to produce the same numbers, you can't really compare policies — any difference might just be noise. Seeding the RNG turns the randomness into a controlled variable.

**Why one RNG per worker.** An `StdRng` is *not* thread-safe by default, and you don't want it to be — the contention would ruin the randomness. Giving each worker its own RNG (seeded deterministically from its `worker_id`) means every run produces the same task sequence per worker, but workers are independent of each other.

**Analogy.** This is the difference between "flip a coin" and "read the next bit off this specific list of coin-flips we wrote down yesterday." Both feel random; only the second lets you replay the experiment.

---

### Step 10 — The CPU budget gate (`monitor11.rs`)

```rust
const CPU_BUDGET: usize = 100;

// before running the task:
loop {
    let result = worker_cpu.fetch_update(
        Ordering::Relaxed,
        Ordering::Relaxed,
        |current| {
            if current + cost <= CPU_BUDGET {
                Some(current + cost)   // reservation succeeded
            } else {
                None                    // no room, refuse
            }
        },
    );
    if result.is_ok() { break; }
    thread::sleep(Duration::from_millis(5));   // back off
}

// task runs here

worker_cpu.fetch_sub(cost, Ordering::Relaxed);
```

**What's new.** Workers now *ask permission* before consuming CPU. If granting the request would push total usage past 100%, the request is denied and the worker waits briefly before trying again.

**Why `fetch_update` and not `fetch_add`.** `fetch_add` unconditionally increments. That's what we want when measuring; that's not what we want when enforcing a cap. `fetch_update` takes a closure: "here's the current value; decide what to write back, or return `None` to refuse." The closure runs atomically — no two workers can both observe "current = 85" and both successfully add 35 to reach 120. One wins the race; the other retries.

**What this guarantees.** `cpu_usage` is never observed above 100. Full stop.

**What this does NOT guarantee.** Fairness. Two workers spinning in the retry loop are unordered — whichever one happens to call `fetch_update` at the right instant wins. A worker trying to reserve 35% can starve behind a stream of 10% IO tasks. That's a real concern, and it's one of the "where could starvation happen" questions your report needs to address.

**Analogy.** The parking garage with a "FULL" sign out front. You don't get to drive in and sort it out; the gate refuses you at the entrance. You circle the block and try again.

---

### Step 11 — Config struct for experiments (`monitor12.rs`)

```rust
#[derive(Clone, Debug)]
struct Config {
    num_workers: usize,
    tasks_per_worker: usize,
    io_probability: f64,
    duration_min_ms: u64,
    duration_max_ms: u64,
    cpu_budget: usize,
    monitor_tick_ms: u64,
    rng_base_seed: u64,
}

impl Config {
    fn balanced() -> Self { /* ... */ }
    fn stressed() -> Self { /* ... */ }
}

fn run_simulation(config: &Config) { /* ... */ }
```

**What's new.** All the tunable numbers are pulled into one place. The simulation is wrapped in a function. Multiple workloads can run back-to-back.

**Why this matters more than it looks.** Experiments A and B in the handout don't differ in *code*, they differ in *parameters*. If your parameters are scattered across `const`s and magic numbers inline in the code, "run experiment B" means editing four places and hoping you got them all. With `Config`, it means changing one line in `main`.

This is also where your FIFO-vs-optimized comparison will live. Same `Config`, two policies. Or same policy, two `Config`s. Either way, one variable at a time, held everything else equal. That's how experiments actually work.

**Feynman moment.** *"A good experiment isolates one thing."* A `Config` struct is the mechanical enforcement of that principle.

---

## Part 3 — The Big Picture, Now That You've Seen the Pieces

Let's tie it back together.

Your final project has these threads running at the same time:

- **1 main thread** — sets up, spawns everyone, waits, prints the final report.
- **1 generator thread** *(or main does it)* — creates tasks and pushes them into the system.
- **1 manager / dispatcher thread** — receives tasks, decides who gets what, respects the CPU budget.
- **8 worker threads** — pull tasks, run them (sleep), release their budget.
- **1 monitor thread** — samples the shared state every 10 ms, accumulates averages, reports at the end.

That's 10 or 11 threads total, which matches the amendment document (*"10 cores... 1 main, 1 monitor, 8 workers"*).

The monitor's relationship to the rest of the system is now clear:

- It **reads** `cpu_usage`, `active_workers`, `completed` — all shared atomics the workers publish to.
- It **owns** its own tally counters (`cpu_sum`, `active_sum`, `sample_count`) — plain locals, no sharing needed.
- It **communicates** with main exactly twice: at startup (main gives it `Arc`s to watch), and at shutdown (it sends a `MonitorReport` back through a channel).

If you remember nothing else, remember this: **the monitor is a tally-keeping lifeguard.** It doesn't swim, it doesn't whistle, it doesn't rescue. It counts. At the end of the shift, it hands in a slip.

---

## Part 4 — Questions

Work through these at your own pace. The answers are at the bottom — try each question before peeking. The point isn't to be right the first time; it's to *notice* what confused you.

### Exercises

**Exercise 1 — The ticking metronome**
Start from `monitor2.rs`. Change the monitor so it prints the word `"tick"` exactly 10 times, 25 ms apart. What happens if you set the main thread's sleep to 50 ms instead of 250 ms? Why?

**Exercise 2 — Why `Arc`?**
In `monitor3.rs`, try removing `Arc` and just passing `completed` directly into both closures. Read the compiler error carefully. What is Rust protecting you from?

**Exercise 3 — The forgotten flag**
In `monitor4.rs`, comment out the line `shutdown.store(true, Ordering::Relaxed);` and run the program. Describe what you see. Now uncomment it. Which handout requirement would a program with this bug violate?

**Exercise 4 — Two signals, one story**
Using `monitor5.rs`, construct a situation (by hand — change the workload in the worker loop) where the monitor prints `cpu=10%` but a task is clearly running. Explain how that's possible and why it's fine.

**Exercise 5 — Add a metric**
Start from `monitor7.rs`. Add a fourth atomic called `max_cpu_seen` that tracks the highest CPU value the monitor has ever observed. Print it in the final report. Hint: the monitor should update it with `fetch_max`.

**Exercise 6 — The lying counter**
In `monitor9.rs`, replace `AtomicUsize` for `completed` with a plain `Mutex<usize>`. Keep everything else the same. Run it. Then find the spot where you're now locking on every single worker task *and* every single monitor tick. If you had 500 tasks and a 10 ms monitor tick over a 5-second run, roughly how many lock acquisitions per second is that? What's the argument for keeping the atomic?

**Exercise 7 — Breaking the budget**
In `monitor11.rs`, change the gate from `fetch_update` to a plain `fetch_add` (i.e., remove the gate entirely). Set `io_probability` to 0.0 so all tasks are CPU (35%). Run with 4 workers. What does `cpu_usage` peak at in the monitor output? Why did the budget matter?

**Exercise 8 — The two-experiment comparison**
Using `monitor12.rs`, run both `Config::balanced()` and `Config::stressed()`. Record `Total time` and `Average CPU` for each. Write *two sentences* — no more — explaining what the difference tells you about your system. (This is literally what your report needs for Experiments A and B.)

**Exercise 9 — Where would starvation hide?**
Look at the admission gate in `monitor11.rs`. Imagine workers A, B, C, D. A and B want to reserve 35% each (CPU tasks). C and D are reserving 10% each (IO tasks) over and over. If current usage is at 70%, who gets in next? Is there any mechanism in the current code to *prevent* A and B from waiting forever while C and D keep squeezing in? What kind of fix would you add?

**Exercise 10 — Explain it out loud**
Without looking at the code, tell your lab partner (or a rubber duck) what happens, in order, from the moment a CPU task is created until the moment the monitor's final report includes it. Mention: the budget gate, the three atomics, the worker's sleep, the monitor's tick, the shutdown handshake, the channel send.

---

### Answers

**Answer 1.** Change the loop to `for _ in 0..10` and the sleep to `Duration::from_millis(25)`. With `main` sleeping only 50 ms, main calls `monitor.join()` while the monitor is still inside its loop — so main *waits* for the monitor (that's what `join` does). The monitor prints 10 times regardless; `join` doesn't kill it, it waits for it. Main sleeping less just means main spent less time before starting to wait.

**Answer 2.** Rust forbids it. You can't `move` the same value into two closures — only one closure can own it. And you can't pass it by reference either, because the compiler can't prove the references will outlive the threads. `Arc` is the workaround: it *is* cheap to clone, and each clone is its own owner. Rust is protecting you from a use-after-free bug that in C++ would compile and then segfault at 3 AM.

**Answer 3.** The program hangs forever. Workers finish, main reaches `monitor.join()`, and the monitor is still spinning inside `while !shutdown.load(...)` because nobody ever set it. You'd have to `Ctrl+C` to exit. This violates the handout's **"no hanging program at the end"** clean-shutdown requirement, which is worth points.

**Answer 4.** An IO task's cost is 10%. Change the worker so it only emits IO tasks. Now while an IO task is running, the monitor will absolutely print `cpu=10%` — and still a task is running. This is fine because "CPU usage" and "task is active" are different questions. `active_workers` would correctly show the task; `cpu_usage` is reporting the resource consumed, which is low. Both are true simultaneously.

**Answer 5.** Add `let max_cpu_seen = Arc::new(AtomicUsize::new(0));`. Clone it for the monitor. Each tick: `max_cpu_seen.fetch_max(cpu, Ordering::Relaxed);`. At the end: `let peak = max_cpu_seen.load(Ordering::Relaxed);` and print it. `fetch_max` is exactly the "update only if bigger" operation you need — and yes, it's a single atomic op, not a load-then-store race.

**Answer 6.** Rough math: 500 workers tasks × 2 lock ops each (acquire + release) = 1000 on the worker side. Monitor ticks at 10 ms over 5 s = 500 ticks × 2 = 1000 more. Call it roughly 400 lock operations per second. That's not catastrophic, but it's pure contention — every lock forces threads to serialize. The atomic version does the same work with a single hardware instruction and no thread ever blocks. *Use atomics for counters. Reserve `Mutex` for structured data you need to read and modify together.*

**Answer 7.** With 4 CPU workers each reserving 35% and no gate, `cpu_usage` peaks at `4 × 35 = 140%`. This is the bug the budget prevents. In our simulation, 140% is just a number — but in a real system, it would represent *oversubscription*: promising more CPU than you have, which causes thrashing, missed deadlines, and angry users. The gate enforces the invariant that made the simulation meaningful in the first place.

**Answer 8.** Example answer shape (your numbers will vary): *"Balanced finished in 420 ms with average CPU around 45%, while stressed took 680 ms with average CPU around 85%. The stressed workload keeps workers closer to the CPU budget ceiling, which is exactly where my admission gate is forced to delay new tasks — so throughput drops even though utilization is higher."* That's two sentences. Notice it *interprets*; it doesn't just restate the numbers.

**Answer 9.** On current usage of 70%, A (needs 35%) is refused — 70+35 > 100. B also refused. C (needs 10%) is admitted — 70+10 = 80. D is admitted — 80+10 = 90. When a C or D task finishes and releases 10%, we're back to where A still can't fit. If C and D keep cycling quickly, A and B wait forever. This is **starvation**. Classic fixes include: priority-aging (A's priority grows while it waits until it jumps the queue), or reservation (reserve N% exclusively for CPU tasks so IO can't fill the whole budget). The handout explicitly lists this in "where could starvation happen" and your report should mention it.

**Answer 10.** There's no one right script, but a correct one touches all these beats: *A CPU task is created with cost 35. The worker picks it up. Before running, the worker calls `fetch_update` on `cpu_usage`: if current + 35 > 100, refuse and sleep; otherwise reserve. On success, `fetch_add(1)` on `active_workers`. Sleep for the task duration. Then `fetch_sub(35)` on `cpu_usage`, `fetch_sub(1)` on `active_workers`, `fetch_add(1)` on `completed`. Meanwhile, every 10 ms, the monitor reads all three and adds them into its local sums. When the last worker finishes, main captures `total_time`, flips `shutdown`, and the monitor sends its final `MonitorReport` back through the channel. Main computes averages from the report and prints them.* If you can tell that story cold, you can pass the demo.

---

## Part 5 — Quick Reference (The Monitor in 8 Bullets)

Pin this somewhere:

1. The monitor is a **separate thread** whose only job is to observe.
2. It reads **shared atomics** that workers publish to — never modifies them.
3. It keeps its **tally locally** — plain variables, no sharing needed.
4. It sleeps a fixed tick (e.g. 10 ms) between samples.
5. It exits on a **shutdown flag** flipped by main after all workers join.
6. It sends its final report back through a **channel** — one-time handoff.
7. `total_time` is captured **before** the monitor joins, not after.
8. Averages are computed in `f64`, not integer division.

---

## Part 6 — A Word About the Demo

The handout is unambiguous: *"A project that runs but cannot be explained well during demo or defense will lose points."*

The monitor is secretly one of the best parts of your project to demo, because it gives you a running story. While your code executes, you'll see lines like:

```
[monitor  120ms] active=6/8  cpu=85%  completed=42
[monitor  130ms] active=5/8  cpu=65%  completed=48
```

That's a narrative. You can literally point at the screen and say *"you can see here that CPU usage stayed pinned near the budget, which is why throughput dropped in this window — it was the gate refusing new reservations."* 
---

*Good luck. Build small, build in layers, and when something confuses you, go back one step.*
