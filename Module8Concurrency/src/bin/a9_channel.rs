fn main() {
    extern crate rand; // 0.8.5

    use std::thread;
    // multi-producer, single consumer
    use std::sync::mpsc::channel;

    let (sender, receiver) = channel();

    for i in 0..10 {
        let sender = sender.clone();
        thread::spawn(move || {
            println!("sending: {}", i);
            sender.send(i).unwrap(); // any data could be passed to receiver
            // as well as sending could fail
        });
    }

    for _ in 0..15 {
        let msg = receiver.recv().unwrap();
        println!("received {}", msg);
    }
    // what is important to notice, data will be sent and received in random order
    // but you will get them in exact order, just be aware of potential queue

    // basically CPU whim
}
