use std::collections::VecDeque;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

#[derive(Debug)]
struct Task {
    id: u32,
}

fn main() { //  channel with queue, manager
    let (tx, rx) = mpsc::channel::<Task>();

    let manager = thread::spawn(move || {
        let mut queue: VecDeque<Task> = VecDeque::new();

        for task in rx {
            println!("    [manager] received task {}, parking in queue", task.id);
            queue.push_back(task);
            println!("    [manager] queue now: {:?}", queue);
        }

        println!("    [manager] channel closed, draining queue:");
        while let Some(task) = queue.pop_front() {
            println!("    [manager] would dispatch task {}", task.id);
        }
        println!("    [manager] done");
    });

    for id in 1..=5 {
        let task = Task { id };
        println!("[generator] sending task {}", task.id);
        tx.send(task).unwrap();
        thread::sleep(Duration::from_millis(20));
    }
    println!("[generator] done");
    drop(tx);

    manager.join().unwrap();
}