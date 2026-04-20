use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

fn main() { // "work stealing" ~load balancing
    // Each number is "how long this task takes"
    let q0 = Arc::new(Mutex::new(VecDeque::from(vec![700, 500, 400, 300])));
    let q1 = Arc::new(Mutex::new(VecDeque::from(vec![100])));

    let queues = vec![Arc::clone(&q0), Arc::clone(&q1)];

    let mut handles = Vec::new();

    for id in 0..2 {
        let my_queue = Arc::clone(&queues[id]);
        let other_queue = Arc::clone(&queues[1 - id]);

        let handle = thread::spawn(move || loop {
            // First try to get work from my own queue
            let my_task = {
                let mut q = my_queue.lock().unwrap();
                q.pop_front()
            };

            if let Some(ms) = my_task {
                println!("worker {id}: doing own task {ms} ms");
                thread::sleep(Duration::from_millis(ms));
                continue;
            }

            // No local work left: try to steal from the OTHER worker
            let stolen = {
                let mut q = other_queue.lock().unwrap();
                q.pop_back() // steal from the far end
            };

            if let Some(ms) = stolen {
                println!("worker {id}: stole task {ms} ms");
                thread::sleep(Duration::from_millis(ms));
                continue;
            }

            // Nothing anywhere
            println!("worker {id}: no work left, exiting");
            break;
        });

        handles.push(handle);
    }

    for h in handles {
        h.join().unwrap();
    }
}