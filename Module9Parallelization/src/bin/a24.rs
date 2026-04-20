use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

fn main() { // simple deque example worker pool
    let tasks = Arc::new(Mutex::new(VecDeque::from(vec![
        700, 100, 500, 200, 400, 300,
    ])));

    let mut handles = Vec::new();

    for worker_id in 0..3 {
        let tasks = Arc::clone(&tasks);

        handles.push(thread::spawn(move || loop {
            let job = {
                let mut q = tasks.lock().unwrap();
                q.pop_front()
            };

            match job {
                Some(ms) => {
                    println!("worker {worker_id}: processing {ms} ms");
                    thread::sleep(Duration::from_millis(ms));
                }
                None => {
                    println!("worker {worker_id}: done");
                    break;
                }
            }
        }));
    }

    for h in handles {
        h.join().unwrap();
    }
}