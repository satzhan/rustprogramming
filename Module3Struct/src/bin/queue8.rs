use std::collections::VecDeque;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy)]
enum Kind {
    Io,
    Cpu,
}

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
        Task {
            id,
            kind: Kind::Io,
            cpu_cost: 10,
            duration: Duration::from_millis(200),
            arrival_time: Instant::now(),
        }
    }

    fn new_cpu(id: u32) -> Self {
        Task {
            id,
            kind: Kind::Cpu,
            cpu_cost: 35,
            duration: Duration::from_millis(200),
            arrival_time: Instant::now(),
        }
    }
}

fn main() { // task kinds
    let (gen_tx, gen_rx) = mpsc::channel::<Task>();
    let (work_tx, work_rx) = mpsc::channel::<Task>();
    let (ready_tx, ready_rx) = mpsc::channel::<()>();

    let manager = thread::spawn(move || {
        let mut queue: VecDeque<Task> = VecDeque::new();
        let mut generator_done = false;

        loop {
            match gen_rx.try_recv() {
                Ok(task) => {
                    println!("    [manager] received task {} ({:?}, {}% cpu)",
                        task.id, task.kind, task.cpu_cost);
                    queue.push_back(task);
                }
                Err(mpsc::TryRecvError::Empty) => {}
                Err(mpsc::TryRecvError::Disconnected) => {
                    generator_done = true;
                }
            }

            if let Ok(()) = ready_rx.try_recv() {
                if let Some(task) = queue.pop_front() {
                    println!("    [manager] dispatching task {}", task.id);
                    work_tx.send(task).unwrap();
                }
            }

            if generator_done && queue.is_empty() {
                break;
            }

            thread::sleep(Duration::from_millis(1));
        }
        println!("    [manager] done");
    });

    let worker = thread::spawn(move || {
        loop {
            if ready_tx.send(()).is_err() {
                break;
            }
            match work_rx.recv() {
                Ok(task) => {
                    let waited = task.arrival_time.elapsed();
                    println!("        [worker] running task {} ({:?}), waited {:?}",
                        task.id, task.kind, waited);
                    thread::sleep(task.duration);
                }
                Err(_) => break,
            }
        }
        println!("        [worker] done");
    });

    let mix = [Kind::Io, Kind::Cpu, Kind::Io, Kind::Io, Kind::Cpu];
    for (i, kind) in mix.iter().enumerate() {
        let id = (i + 1) as u32;
        let task = match kind {
            Kind::Io => Task::new_io(id),
            Kind::Cpu => Task::new_cpu(id),
        };
        println!("[generator] sending task {} ({:?})", task.id, task.kind);
        gen_tx.send(task).unwrap();
        thread::sleep(Duration::from_millis(20));
    }
    drop(gen_tx);

    manager.join().unwrap();
    worker.join().unwrap();
}