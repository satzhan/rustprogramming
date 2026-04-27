use std::sync::mpsc;
use std::thread;
use std::time::Duration;

#[derive(Debug)]
struct Task {
    id: u32,
}

fn main() { // simple send rec
    let (tx, rx) = mpsc::channel::<Task>();

    let consumer = thread::spawn(move || {
        for task in rx {
            println!("    [consumer] got task {}", task.id);
        }
        println!("    [consumer] channel closed — exiting");
    });

    for id in 1..=5 {
        let task = Task { id };
        println!("[main] sending task {}", task.id);
        tx.send(task).unwrap();
        thread::sleep(Duration::from_millis(20));
    }
    println!("[main] done — dropping tx");
    drop(tx);

    consumer.join().unwrap();
}