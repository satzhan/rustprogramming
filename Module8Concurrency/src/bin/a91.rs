use std::thread;
use std::sync::mpsc::channel;

fn main() {
    let (sender, receiver) = channel();

    for i in 0..10 {
        let sender = sender.clone();
        thread::spawn(move || {
            println!("sending: {}", i);
            sender.send(i).unwrap(); 
        });
    }

    drop(sender); 
    for msg in receiver {
        println!("received {}", msg);
    }
}