use std::thread;
use std::sync::mpsc::channel;
use std::time::Duration;
use rand::Rng; 
fn main() {
    let (sender, receiver) = channel();

    for i in 0..10 {
        let sender = sender.clone();
        thread::spawn(move || {
            let random_time = rand::thread_rng().gen_range(10..=100);
            thread::sleep(Duration::from_millis(random_time));
            println!("Thread {} done!", i);
            sender.send(i).unwrap(); 
        });
    }

    drop(sender);

    for msg in receiver {
        println!("Main thread received: {}", msg);
    }
}