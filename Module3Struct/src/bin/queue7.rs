use std::collections::VecDeque;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

#[derive(Debug)]
struct Task {
    id: u32,
}

fn main() { // manager worker generator
    let (gen_tx, gen_rx) = mpsc::channel::<Task>();
    let (work_tx, work_rx) = mpsc::channel::<Task>();
    let (ready_tx, ready_rx) = mpsc::channel::<()>();

    let manager = thread::spawn(move || {
        let mut queue: VecDeque<Task> = VecDeque::new();
        let mut generator_done = false;

        loop {
            match gen_rx.try_recv() {
                Ok(task) => {
                    println!("    [manager] received task {}", task.id);
                    queue.push_back(task);
                    println!("Current queue: {:?}", queue);
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
                println!("    [manager] all done, exiting");
                break;
            }

            thread::sleep(Duration::from_millis(1));
        }
    });

    let worker = thread::spawn(move || {
        for _ in 0..5 {
            ready_tx.send(()).unwrap();
            let task = work_rx.recv().unwrap();
            println!("        [worker] executing task {}", task.id);
            thread::sleep(Duration::from_millis(150));
        }
    });

    for id in 1..=5 {
        gen_tx.send(Task { id }).unwrap();
        println!("[generator] sent task {}", id);
        thread::sleep(Duration::from_millis(50));
    }
    drop(gen_tx);

    manager.join().unwrap();
    worker.join().unwrap();
}