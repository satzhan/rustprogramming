# A Walk Through the Concurrent Task Dispatcher

A guided tour of the eleven warm-up programs that lead into the final project. Each one adds *exactly one new idea* to the previous one, so you can read them in order and never have to swallow more than one new concept at a time.

The point of this guide is not to drop a finished dispatcher in your lap. It's to make every line of the final design feel inevitable — like the only sensible answer to a question the previous program quietly raised.

---

## How to read this guide

Each section follows the same shape:

- **What's new.** The single idea this program adds.
- **The snippet.** The smallest piece of code that shows the idea.
- **Why it's there.** What problem the previous program left unsolved.
- **What it costs.** Every change pays for something with something else.

If you find yourself nodding along without writing any code, stop and run the example. Concurrency is one of those subjects where the difference between *understanding* and *thinking you understand* is exactly one debugger session.

---

## Part 1 — The deque, before any threads exist

Before tasks, before workers, before channels: a queue is just a place to put things and take them back out. The first four programs are entirely single-threaded. They exist to make sure you have a clean mental model of the data structure that will eventually sit at the heart of the manager.

### 1.1 — `queue1.rs`: push back, pop front

The simplest possible queue. People walk up to a coffee counter, get added to the line at the back, and the barista serves whoever is standing at the front.

```rust
let mut queue: VecDeque<String> = VecDeque::new();

queue.push_back(String::from("Alice"));
queue.push_back(String::from("Bob"));
queue.push_back(String::from("Carol"));

while let Some(person) = queue.pop_front() {
    println!("Serving: {}", person);
}
```

Two operations, two ends, FIFO order. That's the whole contract.

A `VecDeque<T>` is a *double-ended* queue, meaning you can push and pop from either end. You only need one end here, but the type is built to support both, and that flexibility will matter later when the manager wants to peek at the front without taking the item off.

**What it costs:** nothing. There's no concurrency story yet, no ownership puzzle, no shared state. This is the baseline against which every later program will look more complicated.

### 1.2 — `queue2.rs`: same idea, more elements, watch it grow

Same shape as the first program, but now you push ten items in a loop and then drain them. The interesting part isn't the code, it's what `cargo run` prints.

```rust
for n in 1..=10 {
    queue.push_back(n);
    println!("Pushed {} to back. Queue: {:?}", n, queue);
}
```

Run it. The queue is reported the same way each time, but the *capacity* (which the debug print hides) doubles behind your back: roughly 4 → 8 → 16. That doubling is not arbitrary.

If you grew the buffer by one slot every time it filled up, pushing N items would cost N + (N−1) + (N−2) + … work — quadratic. By doubling instead, each item gets copied at most about once on average, no matter how many you push. That's the trick called *amortized constant time*: any single push might be expensive, but the average cost across many pushes stays flat.

This becomes relevant the moment your generator starts producing 1000 tasks. You don't want a hidden quadratic lurking in your queue.

### 1.3 — `queue3.rs`: a real use case (LeetCode 933)

The "RecentCounter" problem: count how many pings have come in within the last 3000 milliseconds.

```rust
fn ping(&mut self, t: i32) -> i32 {
    self.pings.push_back(t);
    while let Some(&front) = self.pings.front() {
        if front < t - 3000 {
            self.pings.pop_front();
        } else {
            break;
        }
    }
    self.pings.len() as i32
}
```

This is the first program that uses *both* ends of the deque, and it's worth pausing on why a deque fits this problem so naturally:

- Pings arrive in chronological order, so new ones go on the **back**.
- Pings expire in chronological order too — the *oldest* one is always the one that times out first — so they leave from the **front**.

If you tried to do this with a `Vec<i32>` and `remove(0)`, every expiration would cost O(n) because removing from the front of a `Vec` shifts every other element down. The deque does it in O(1) by moving an index, not by moving data.

**The pattern to remember:** "things arrive at one rate and depart in the same order at another rate" is exactly the shape of a queue. The dispatcher you'll build is a much fancier version of this same situation.

### 1.4 — `queue4.rs`: building your own deque

Here's where the abstraction cracks open. The earlier programs treated `VecDeque` as a black box. This one rebuilds it from scratch, badly enough to be readable and well enough to actually work.

```rust
struct MyDeque<T> {
    buf: Vec<Option<T>>,  // backing storage
    head: usize,          // index of the logical front
    len: usize,           // how many slots are occupied
    cap: usize,           // total slots
}
```

The mental model that surprises most people is this: **a deque is not built out of "two pointers." It's built out of one flat array and two integers.** The integers tell you which slot is currently the front and how many slots are in use. The "back" is computed as `(head + len) % cap`. The wraparound makes the array act like a ring.

Here's what `push_back` actually does:

```rust
fn push_back(&mut self, item: T) {
    if self.len == self.cap { self.grow(); }
    let slot = (self.head + self.len) % self.cap;
    self.buf[slot] = Some(item);
    self.len += 1;
}
```

That `% self.cap` is the whole secret. As long as `cap` is a power of two (4, 8, 16, …), the modulo is essentially free at the CPU level — it collapses into a single bitmask. That's why the real `VecDeque` is more committed to powers of two than `Vec` is: it pays a little wasted memory for indexing math that costs almost nothing.

A useful image:

> A `Vec` is a one-way escalator. You can only board at the bottom and exit at the top. When it fills, you build a taller escalator and re-place everyone in the same order.
>
> A `VecDeque` is a carousel with a movable "front-of-line" sign. Riders board and leave from either end; the carousel itself doesn't move, but the sign does. When all the seats fill up, the operator builds a bigger carousel and re-seats everyone starting from seat 0 — but it's still one continuous ring, not a chain of separate cars.
>
> A `LinkedList`, by contrast, is a freight train: each car is welded to the next by physical couplings. It's lighter to add a car at the end (no rebuilding), but every car carries the weight of two coupling pieces, and walking the train means stepping car-to-car.

The `grow()` function is the carousel-rebuild moment. When the buffer fills, you allocate a new buffer of double the size, copy the elements out *in logical order*, and reset `head = 0`. After growth, the data is "unwound" — no more wraparound until things shift around again.

```rust
fn grow(&mut self) {
    let new_cap = self.cap * 2;
    // ... allocate new buffer ...
    for i in 0..self.len {
        let old = (self.head + i) % self.cap;
        new_buf[i] = self.buf[old].take();
    }
    self.buf = new_buf;
    self.cap = new_cap;
    self.head = 0;
}
```

**The key distinction.** People are taught "queue = singly linked list, deque = doubly linked list," and then they expect a `VecDeque` to have prev/next pointers somewhere. It doesn't. The "doubly" in *double-ended* is about which ends you can touch, not about how the structure is built underneath. `VecDeque`'s two-endedness is bookkeeping on a flat array, not extra plumbing on each item.

Once you've internalized this, the rest of the project becomes much easier to reason about, because every queue you'll see from here on is *that* structure with threads bolted on around it.

---

## Part 2 — Two threads, one queue

The next two programs introduce the thing that makes everything else possible: a way for one thread to hand work to another without sharing memory directly.

### 2.1 — `queue5.rs`: the channel

```rust
let (tx, rx) = mpsc::channel::<Task>();

let consumer = thread::spawn(move || {
    for task in rx {
        println!("    [consumer] got task {}", task.id);
    }
});

for id in 1..=5 {
    tx.send(Task { id }).unwrap();
}
drop(tx);
consumer.join().unwrap();
```

The `mpsc` in `mpsc::channel` stands for *multi-producer, single-consumer*. You can clone the `tx` (sender) end and have many threads send into the same channel; only one thread can hold the `rx` (receiver) end at a time. That asymmetry is built into the type system — you literally cannot accidentally split `rx` across threads.

Three things to notice:

1. **`for task in rx` is the whole receive loop.** A channel implements `IntoIterator`, and the iterator yields tasks until every sender has been dropped. That's why the explicit `drop(tx)` matters — without it, the consumer would wait forever for a sender that no one ever closed.
2. **`move` on the closure.** When you spawn a thread, the closure must own everything it touches, because the parent thread might end first. `move` says "this closure takes ownership of any variables it captures." Here it takes ownership of `rx`.
3. **Send is non-blocking on an unbounded channel.** Calling `tx.send(task)` doesn't wait for the consumer to be ready. The task lives in the channel's internal buffer until someone receives it.

This is the simplest possible producer/consumer system. One sender, one receiver, no queue you can see — but there *is* an internal queue inside the channel itself.

### 2.2 — `queue6.rs`: a manager with memory

A channel by itself buffers tasks but gives you no control over them. You can't peek at them, reorder them, or decide one is more urgent than another. So the next step puts a deque *inside* the receiver:

```rust
let manager = thread::spawn(move || {
    let mut queue: VecDeque<Task> = VecDeque::new();

    for task in rx {
        println!("    [manager] received task {}, parking in queue", task.id);
        queue.push_back(task);
    }

    println!("    [manager] channel closed, draining queue:");
    while let Some(task) = queue.pop_front() {
        println!("    [manager] would dispatch task {}", task.id);
    }
});
```

This is a tiny program but it's where the architecture starts to take shape. The channel has become a *delivery mechanism*; the deque has become *holding storage*. Two roles, two data structures, separated. That separation is what lets the manager eventually do interesting things — like check resource availability before dispatching, or reorder by priority.

**What it costs:** you've added a thread, a layer of indirection, and the responsibility to drain the queue at shutdown. Notice the structure: receive everything until the channel closes, *then* drain. Real systems can't always do this cleanly — sometimes new work arrives at the same time as old work is finishing — but for the warm-up, this draining-after-close pattern works.

---

## Part 3 — Three roles, one pipeline

### 3.1 — `queue7.rs`: generator → manager → worker

The first program with three threads playing three different roles. It's worth taking seriously even though the code looks busy.

```rust
let (gen_tx, gen_rx) = mpsc::channel::<Task>();   // generator → manager
let (work_tx, work_rx) = mpsc::channel::<Task>(); // manager → worker
let (ready_tx, ready_rx) = mpsc::channel::<()>(); // worker → manager
```

Three channels. The first two are obvious — they carry tasks forward through the pipeline. The third one is the *interesting* one: it carries empty `()` values *backward*, from the worker to the manager, signaling "I am ready for another task."

Why does that backward channel exist? Because if the manager just shoved tasks at the worker as fast as they came in, you'd lose the whole point of having a manager. The manager wants to dispatch *only when there's somewhere to dispatch to*. The ready signal is the worker raising its hand.

The manager's loop now does three things at once:

```rust
loop {
    // 1. Try to receive new work from the generator (non-blocking)
    match gen_rx.try_recv() {
        Ok(task) => queue.push_back(task),
        Err(mpsc::TryRecvError::Empty) => {}
        Err(mpsc::TryRecvError::Disconnected) => generator_done = true,
    }

    // 2. If a worker raised its hand, hand it the next task
    if let Ok(()) = ready_rx.try_recv() {
        if let Some(task) = queue.pop_front() {
            work_tx.send(task).unwrap();
        }
    }

    // 3. If the generator is done and the queue is empty, exit
    if generator_done && queue.is_empty() { break; }

    thread::sleep(Duration::from_millis(1));
}
```

The `try_recv` calls are the key change from earlier programs. `recv()` blocks the thread until something arrives. `try_recv()` returns immediately — either with a value, with `Empty` (nothing to read), or with `Disconnected` (the sender is gone forever).

A blocking call would be wrong here, because the manager has *two* things it might react to (incoming tasks, ready signals) and can't afford to sit idle on either one. The 1-millisecond sleep at the bottom prevents the loop from burning the CPU by spinning at full speed when both channels are empty.

This non-blocking-loop-with-tiny-sleep shape is a *polling* pattern. It's not the most elegant solution — there are condition variables and `select!`-style primitives that handle this more cleanly — but it's easy to read, easy to reason about, and good enough for a simulation.

**The key shift in mindset:** earlier programs had one thread reacting to one source of events. From here on, the manager reacts to multiple sources, and the question "what should the manager do right now?" becomes the central design question of the whole project.

---

## Part 4 — Tasks become real things

### 4.1 — `queue8.rs`: a Task struct with kind, cost, duration, arrival

Up to now, a task has been an ID and nothing else. The amendment document is specific: tasks have a *kind* (CPU or IO), a *cpu_cost*, a *duration*, and an *arrival time*. Modeling that explicitly is the next step:

```rust
#[derive(Debug, Clone, Copy)]
enum Kind { Io, Cpu }

#[derive(Debug)]
struct Task {
    id: u32,
    kind: Kind,
    cpu_cost: u8,
    duration: Duration,
    arrival_time: Instant,
}

impl Task {
    fn new_io(id: u32) -> Self {
        Task { id, kind: Kind::Io, cpu_cost: 10,
               duration: Duration::from_millis(200),
               arrival_time: Instant::now() }
    }
    fn new_cpu(id: u32) -> Self {
        Task { id, kind: Kind::Cpu, cpu_cost: 35,
               duration: Duration::from_millis(200),
               arrival_time: Instant::now() }
    }
}
```

A few things worth lingering on:

**Why an `enum`, not a `bool` or a string?** The kind is a *closed set* — there are exactly two possibilities, and you want the compiler to force you to handle both whenever you switch on it. A `bool` would lose the names. A `String` would let you mistype `"cpu"` and ship a bug. `enum` gives you names and exhaustiveness checking, which is the Rust style for "this thing is one of a small known set of cases."

**Why `Instant`, not a `u64` of milliseconds?** `Instant` is a monotonic clock — it never goes backward, even if the system clock is adjusted. For measuring elapsed time, that's exactly what you want. Subtracting two `Instant`s gives you a `Duration`, and `Duration` knows how to print itself, compare itself, and convert to milliseconds when you need to.

**Why bake `arrival_time` into the task at construction?** Because the wait time of a task is "now minus arrival time" *at the moment the worker picks it up*. If you don't capture arrival at creation, you've thrown away the information you'll need for metrics later. Once a task is in flight, it carries its own history with it.

The worker now has something interesting to print:

```rust
let waited = task.arrival_time.elapsed();
println!("    [worker] running task {} ({:?}), waited {:?}",
    task.id, task.kind, waited);
thread::sleep(task.duration);
```

`task.arrival_time.elapsed()` is shorthand for "now minus the moment this `Instant` was captured." That's the wait time — exactly the metric the project asks you to report.

---

## Part 5 — One worker becomes a pool

### 5.1 — `queue9.rs`: many workers, one shared receiver

The previous program had one worker. The amendment says eight. Adding more is trickier than it sounds, because of how `mpsc` is structured: there's only *one* receiver, and only one thread can hold it at a time.

The standard fix:

```rust
let (work_tx, work_rx) = mpsc::channel::<Task>();
let work_rx = Arc::new(Mutex::new(work_rx));
```

Two new types appear here, both load-bearing:

- **`Arc<T>`** — *Atomically Reference Counted*. It's a smart pointer that lets multiple threads share ownership of the same `T`. The "atomic" part is what makes it safe across threads: incrementing the reference count is a single atomic CPU operation, not a read-modify-write that could race.
- **`Mutex<T>`** — *Mutual exclusion*. A lock. Only one thread can hold it at a time. `lock()` blocks until the lock is free, then returns a guard; when the guard goes out of scope, the lock is released automatically.

Together, `Arc<Mutex<T>>` is the canonical Rust pattern for "this thing is shared and mutable." The `Arc` lets multiple threads point at it; the `Mutex` makes sure they take turns.

Each worker's loop now looks like this:

```rust
loop {
    if ready_tx.send(()).is_err() { break; }
    let task = {
        let rx = work_rx.lock().unwrap();
        rx.recv()
    };
    match task {
        Ok(task) => {
            // run the task
        }
        Err(_) => break,
    }
}
```

The block around `rx.lock()` and `rx.recv()` is intentional. The lock guard is held only as long as the inner block runs, so the worker releases the lock the instant it has a task in hand. If you held the lock while *running* the task too, only one worker could ever be active at a time — you'd have eight workers behaving like one.

**The lesson:** hold a lock for the smallest possible window. The lock exists to protect the *receiver*, not the work. Once you've fished a task out of the channel, the lock has done its job and should let go.

There's also a subtle ownership trick in the worker spawn loop: the receiver `Arc` and the `ready_tx` sender are both *cloned per worker*. Each clone bumps the reference count; each worker thread takes ownership of its own clone. None of them owns the original.

---

## Part 6 — Counting instead of asking

### 6.1 — `queue10.rs`: replacing the ready-signal with an atomic counter

The ready channel from earlier worked, but it has an awkward property: every dispatch requires a round trip. The worker says "ready," the manager hears "ready," the manager sends a task, the worker receives it. Three messages cross thread boundaries for every single task. That's overhead that scales linearly with the number of tasks.

There's a cleaner model: instead of asking workers whether they're available, *count* them.

```rust
let active_workers = Arc::new(AtomicUsize::new(0));
```

An `AtomicUsize` is a `usize` that supports thread-safe operations without a lock. You can increment it, decrement it, read it, and compare-and-swap it, all atomically. It's faster than a `Mutex<usize>` because it skips the locking machinery — these operations compile down to a handful of CPU instructions.

The worker now updates the counter directly:

```rust
match task {
    Ok(task) => {
        active.fetch_add(1, Ordering::AcqRel);  // I'm starting a task
        // ... run task ...
        active.fetch_sub(1, Ordering::AcqRel);  // I'm done
    }
    Err(_) => break,
}
```

And the manager checks it before dispatching:

```rust
if manager_active.load(Ordering::Acquire) < NUM_WORKERS {
    if let Some(task) = queue.pop_front() {
        work_tx.send(task).unwrap();
    }
}
```

The ready channel is gone entirely. The information it used to carry — "is anyone free?" — is now stored in a single integer that any thread can read without coordination.

A few things worth understanding about the `Ordering` parameter:

- **`Acquire`** on a load means "I want to see all writes that happened before the corresponding `Release`." Use it when reading a value that other threads might have just written.
- **`Release`** on a store means "make my writes visible to anyone who does an `Acquire` load after this."
- **`AcqRel`** on a read-modify-write (like `fetch_add`) means both at once — acquire on the read, release on the write.

You can usually get away with `Ordering::SeqCst` (the strongest, slowest option) and not think about it. The acquire/release pair is more efficient and is correct for this pattern, where the manager reads what the workers wrote.

**What changed structurally:** the system is now *push-based with capacity awareness* rather than *pull-based with handshakes*. The manager dispatches whenever there's room; workers signal availability passively, by being one of the slots that isn't currently incremented. Fewer messages, less coupling, simpler shutdown logic.

---

## Part 7 — Resources have ceilings

### 7.1 — `queue11.rs`: the CPU cap and the hold/release dance

This is the program that actually implements the amendment's hard rule: **global CPU usage can't exceed 100%.**

If a CPU task costs 35% and an IO task costs 10%, you can't just dispatch greedily — you might assign three CPU tasks (105%) and overshoot. You also can't *only* count workers, because workers are a different resource than CPU. A free worker doesn't help if there's no CPU budget for the next task.

The shape of the answer:

```rust
const NUM_WORKERS: usize = 4;
const CPU_CAP: usize = 100;

let active_workers = Arc::new(AtomicUsize::new(0));
let cpu_load = Arc::new(AtomicUsize::new(0));
```

Two atomic counters now. Workers update both:

```rust
active.fetch_add(1, Ordering::AcqRel);
cpu.fetch_add(cost, Ordering::AcqRel);
// run task
cpu.fetch_sub(cost, Ordering::AcqRel);
active.fetch_sub(1, Ordering::AcqRel);
```

And the manager has to *peek before popping*, because dispatching a task it can't fit would corrupt the queue order:

```rust
if let Some(front) = queue.front() {
    let workers_in_use = manager_active.load(Ordering::Acquire);
    let cpu_in_use = manager_cpu.load(Ordering::Acquire);
    let cost = front.cpu_cost as usize;

    let worker_ok = workers_in_use < NUM_WORKERS;
    let cpu_ok = cpu_in_use + cost <= CPU_CAP;

    if worker_ok && cpu_ok {
        let task = queue.pop_front().unwrap();
        work_tx.send(task).unwrap();
    } else {
        // hold — print why, but don't pop
    }
}
```

This is where the deque's *two-ended* nature finally pays off. `front()` returns a reference to the next task without removing it. The manager looks at it, decides whether it fits, and either commits (`pop_front`) or holds. With a single-ended queue you'd be stuck — you can't tell if the next task fits without peeking, and you can't peek without taking.

There's one more piece worth understanding: the **hold state**.

```rust
enum HoldState {
    Idle,
    Holding { task_id: u32 },
}
```

Without this, the manager would print "task X is being held" once per loop iteration — hundreds of times per second — for every millisecond the task sat blocked. The hold state remembers *which* task is currently waiting, and only logs the transition into and out of holding. That's not a concurrency feature; it's just polite to the log file. But it's a great example of how a tiny piece of state can turn a noisy system into a readable one.

**What this program actually demonstrates:** scheduling is *resource-aware admission control*, not just FIFO. The queue still operates in FIFO order — the front task is always next — but a task is only admitted when the system has the resources to run it. That's the difference between "I have someone to do this" and "I have someone *and* the budget to actually run it."

---

## Looking back at the staircase

If you flip back through the eleven programs and watch what changes from one to the next, the arc is remarkably regular:

| From | To | What was added |
|------|-----|----------------|
| 1 → 2 | Same idea, more elements | Confidence in growth behavior |
| 2 → 3 | Toy example | Real use of both ends of the deque |
| 3 → 4 | Black box | Open-box understanding of the ring buffer |
| 4 → 5 | Single thread | A second thread, connected by a channel |
| 5 → 6 | Channel only | A queue *behind* the channel, owned by a manager |
| 6 → 7 | Two roles | Three roles plus a back-channel for readiness |
| 7 → 8 | Bare task IDs | Tasks with kind, cost, duration, arrival time |
| 8 → 9 | One worker | A worker pool sharing one receiver |
| 9 → 10 | Ready signals | An atomic counter replacing the back-channel |
| 10 → 11 | One resource | Two resources (workers + CPU) with admission control |

Every step adds one idea. None of them invent new pieces from scratch — each one rearranges the previous program's pieces and inserts a single new mechanism. By the time you reach the last one, you have all the moving parts of the final dispatcher: a generator, a manager with a deque, a peek-then-pop dispatch decision, a worker pool sharing a receiver, atomic counters tracking shared resources, and clean shutdown when everything has drained.

The final project asks for two simulations: a baseline FIFO version and an optimized version. The baseline is essentially `queue11` scaled up to 8 workers and 1000 tasks. The optimized version is wherever you go from here — reordering tasks to fit the cap better, reserving some workers for IO, weighting the queue, adding aging, anything that improves total runtime or average CPU usage without breaking the resource ceiling.

The question to keep returning to is the one the project ends with: *if someone points at any task in your system and asks where it is, who owns it, and what happens to it next, you should be able to answer.* If you've followed the staircase up from `queue1`, you can. Each program owned one piece of that answer; together, they own the whole thing.
