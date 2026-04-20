use std::collections::VecDeque;
use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use std::time::{Duration, Instant};

#[derive(Debug, Default)]
struct Queues {
    high: VecDeque<u32>,
    low: VecDeque<u32>,
    stop: bool,
}

fn main() {
    let shared = Arc::new((Mutex::new(Queues::default()), Condvar::new()));

    // Producer: keeps flooding high-priority work
    let high_shared = Arc::clone(&shared);
    let high_producer = thread::spawn(move || {
        let (lock, cvar) = &*high_shared;
        let mut id = 0;

        loop {
            thread::sleep(Duration::from_millis(40)); // arrives faster than consumer can drain
            let mut q = lock.lock().unwrap();
            if q.stop {
                break;
            }
            q.high.push_back(id);
            println!("[HIGH PRODUCER] added high task {id}");
            id += 1;
            cvar.notify_one();
        }
    });

    // Producer: adds a few low-priority tasks
    let low_shared = Arc::clone(&shared);
    let low_producer = thread::spawn(move || {
        let (lock, cvar) = &*low_shared;

        for id in 0..5 {
            thread::sleep(Duration::from_millis(120));
            let mut q = lock.lock().unwrap();
            if q.stop {
                break;
            }
            q.low.push_back(id);
            println!("                 [LOW PRODUCER] added low task {id}");
            cvar.notify_one();
        }
    });

    // Consumer: unfair policy -> always serve high first
    let worker_shared = Arc::clone(&shared);
    let worker = thread::spawn(move || {
        let (lock, cvar) = &*worker_shared;

        loop {
            let mut q = lock.lock().unwrap();

            while q.high.is_empty() && q.low.is_empty() && !q.stop {
                q = cvar.wait(q).unwrap();
            }

            if q.stop {
                break;
            }

            if let Some(task) = q.high.pop_front() {
                drop(q);
                println!("[WORKER] processing HIGH task {task}");
                thread::sleep(Duration::from_millis(80));
            } else if let Some(task) = q.low.pop_front() {
                drop(q);
                println!("                 [WORKER] processing LOW task {task}");
                thread::sleep(Duration::from_millis(80));
            }
        }
    });

    // Let the system run for a while
    let start = Instant::now();
    while start.elapsed() < Duration::from_secs(3) {
        thread::sleep(Duration::from_millis(250));

        let (lock, _) = &*shared;
        let q = lock.lock().unwrap();
        println!(
            ">>> STATUS: high queued = {}, low queued = {}",
            q.high.len(),
            q.low.len()
        );
    }

    // Stop everything
    {
        let (lock, cvar) = &*shared;
        let mut q = lock.lock().unwrap();
        q.stop = true;
        cvar.notify_all();
    }

    high_producer.join().unwrap();
    low_producer.join().unwrap();
    worker.join().unwrap();

    // Final state
    let (lock, _) = &*shared;
    let q = lock.lock().unwrap();
    println!("\nFINAL:");
    println!("remaining high tasks = {}", q.high.len());
    println!("remaining low tasks  = {}", q.low.len());
    println!("remaining low queue contents = {:?}", q.low);
}