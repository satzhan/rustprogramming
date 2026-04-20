fn main() {
    extern crate rand;
    use rand::Rng;
    use std::thread;
    // multi-producer, single consumer
    use std::sync::mpsc::channel;
    use std::time;

    let ten_millis = time::Duration::from_millis(1000);
    
    const DISCONNECT: &str = "Come back tomorrow!";
    
    let (sender, receiver) = channel();
    
    thread::spawn(move || {
        let mut rng = rand::thread_rng();
        loop {
            let msg = match rng.gen_range(0..5) {
                0 => "Fortune favors the brave.",
                1 => DISCONNECT,
                2 => "You will travel to many exotic places in your lifetime.",
                3 => "You can make your own happiness.",
                4 => "You are very talented in many ways.",
                _ => unreachable!(),
            };
            
            println!("Sending cookie: {}", msg);
            //thread::sleep(ten_millis);
            sender.send(msg).unwrap();
            if msg == DISCONNECT {
                break;
            }
        }
    });
    
    for received_msg in receiver {
        println!("What a day. Your fortune cookie : {}", received_msg);
        thread::sleep(ten_millis);
    }
}