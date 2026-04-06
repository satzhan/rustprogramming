# Part I: Introduction to Concurrency

## 1. The Core Concept: A Simple Analogy
Imagine a scenario: There are three $100 banknotes on a table and three people instructed to pick them up. 
* If they don't communicate with each other and simply move to pick up the money, the process of distribution becomes a concurrent task.
* The task can **fail** (e.g., two people grab the same note and tear it) if there is no communication. 
* It could also be **inefficient** if they force a rule where only one person can enter the room and take one note at a time.

This provides a simple, basic, but accurate mental model of concurrent programming.

---

## 2. A New Abstraction: The Thread
Historically, a program consisted of a single point of execution. The introduction of threads means our program can now have **multiple points of execution**. 

Think of each thread as an independent process, but with one critical distinction: **All threads within a process share the same address space and the same data.**

### The State of a Single Thread
To maintain its own execution context (so that context switching still works), each thread has its own:
* **Program Counter (PC)**
* **Private Registers**

### Processes vs. Threads Memory Model
To distinguish between threads and allow them to execute independently, each thread is allocated its own individual stack. The stack serves as thread-local storage, keeping track of local variables, parameters, and return values.

**Single Process, Single Thread:**
> `[ Code | Heap | Stack ]`

**Single Process, Multiple Threads:**
> `[ Code | Heap | Stack 1 | Stack 2 | Stack 3 ]`

---

## 3. Why Use Threads?

### 1) Parallelism
Imagine you need to process 10,000 distinct arrays to find their total sum. 
* **Single-threaded:** The program processes one array at a time, sequentially.
* **Multi-threaded:** You can divide the workload. For example, you can spawn 10 threads and assign each thread 1,000 arrays to process simultaneously. This is parallelization.

### 2) Interactivity (Preventing Blocking)
When your program runs on the CPU, certain instructions—like opening a file or waiting for network data—cause the program to block. If you only have one thread, the entire program gives up the CPU and halts, which creates a slow, unresponsive experience. 
Instead of relinquishing the CPU on every I/O operation, you can use multiple threads to ensure the CPU is still doing useful work while one specific thread waits.

---

## 4. Thread Creation Concepts
The two fundamental lifecycle operations for a thread are:
1. **Spawn:** Create and start a new thread.
2. **Join:** Wait for a spawned thread to finish its execution.

### Example in Rust
Here is what happens when we don't properly wait for a thread to finish.

**Bad Example (The spawned thread may not finish before `main` exits):**
```rust
use std::thread;
use std::time::Duration;

fn main() {
    thread::spawn(|| {
        for i in 1..10 {
            println!("hi number {} from the spawned thread!", i);
            thread::sleep(Duration::from_millis(1));
        }
    });

    for i in 1..5 {
        println!("hi number {} from the main thread!", i);
        thread::sleep(Duration::from_millis(1));
    }
}
```

**Correct Example (Using `join` to ensure completion):**
```rust
use std::thread;
use std::time::Duration;

fn main() {
    let handle = thread::spawn(|| {
        for i in 1..10 {
            println!("hi number {} from the spawned thread!", i);
            thread::sleep(Duration::from_millis(1));
        }
    });

    for i in 1..5 {
        println!("hi number {} from the main thread!", i);
        thread::sleep(Duration::from_millis(1));
    }

    // Wait for the spawned thread to finish before exiting main
    handle.join().unwrap();
}
```

---

## 5. The Problems: Shared Data & Scheduling
Because all threads share the data that lives on the heap, problems begin to emerge when multiple threads try to interact with that data simultaneously.

### Uncontrolled Scheduling and Data Races
Consider a simple operation: `a = a + 1`. At the hardware level, this is not a single step. It involves:
1. **Load:** Move the value of `a` into a register.
2. **Update:** Perform the addition (add 1).
3. **Store:** Put the new value back into memory.

A critical failure occurs if a hardware timer interrupt triggers a context switch exactly in the middle of these steps. If two threads try to update the same shared data concurrently without protection, the final result becomes **nondeterministic**. 

This specific issue is known as a **Race Condition** (or Data Race). *Note: Rust's ownership and type system prevent data races at compile time, but understanding the underlying systems concept is essential for deep comprehension.*

---

## 6. Synchronization Primitives

Any section of code that involves updating shared heap data is referred to as a **Critical Section**. To safely navigate critical sections, we need synchronization and controlled access.

### Mutual Exclusion (Mutex)
Pioneered by Edsger Dijkstra, the solution to data races is mutual exclusion.
* **Key Idea:** Only allow one thread to access and modify the shared data at a time.
* **Atomic Operations:** Operations must be "all or nothing." Think of a bank transfer: it consists of a *withdrawal* and a *deposit*. You want both to execute successfully, or neither. They cannot be split.

### Condition Variables
Sometimes, synchronization requires more than just locking data; it requires orchestrating the *order* of execution. 
* If a thread cannot continue its execution until another thread finishes a specific task, it needs a way to go to sleep and be woken up only when the condition is met.
* **Solution:** Condition Variables handle the signaling, sleeping, and waking of appropriate threads. *(Example problem: LeetCode "Print Zero Even Odd").*

---

## 7. Key Takeaways
* **Critical Section:** A block of code that accesses shared resources and must not be executed by more than one thread at a time.
* **Race Condition:** A flaw where the timing or order of thread scheduling changes the program's correctness.
* **Deterministic vs. Nondeterministic:** Predictable output versus output that changes based on uncontrollable system scheduling.
* **Mutex (Mutual Exclusion):** A primitive lock used to prevent simultaneous access to a shared resource.
