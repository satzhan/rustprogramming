use std::sync::mpsc;
use std::thread;
use std::time::Duration;

#[derive(Debug)]
pub enum TaskKind {
    Cpu,
    Io,
}

#[derive(Debug)]
pub struct Task {
    pub id: usize,
    pub kind: TaskKind,
    pub duration: Duration,
}

pub struct Worker {
    pub id: usize,
    pub thread: Option<thread::JoinHandle<()>>,
}

impl Worker {
    pub fn new(id: usize, receiver: mpsc::Receiver<Task>) -> Self {
        let thread = thread::spawn(move || {
            while let Ok(task) = receiver.recv() {
                println!("Worker {} processing Task {}", id, task.id);
                thread::sleep(task.duration);
            }
            println!("Worker {} shutting down.", id);
        });

        Worker {
            id,
            thread: Some(thread),
        }
    }
}

fn main() {
    let num_workers = 4;
    let mut workers = Vec::new();
    let mut senders = Vec::new();

    for id in 0..num_workers {
        let (tx, rx) = mpsc::channel();
        senders.push(tx);
        workers.push(Worker::new(id, rx));
    }

    for i in 0..12 {
        let task = Task {
            id: i,
            kind: if i % 2 == 0 { TaskKind::Cpu } else { TaskKind::Io },
            duration: Duration::from_millis(100),
        };

        let worker_index = i % num_workers;
        senders[worker_index].send(task).unwrap();
    }

    drop(senders);

    for mut worker in workers {
        if let Some(thread) = worker.thread.take() {
            thread.join().unwrap();
        }
    }

    println!("System shutdown complete.");
}