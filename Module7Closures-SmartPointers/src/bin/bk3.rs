use std::rc::Rc;

struct Tv {
    channel: String,
}

impl Drop for Tv {
    fn drop(&mut self) {
        println!("The TV is officially turned off and memory is freed!");
    }
}

fn main() {
    println!("--- House Opens ---");

    let bob: Rc<Tv>;

    {
        println!("--> Alice enters the living room and turns on the TV.");
        let alice = Rc::new(Tv {
            channel: String::from("Sports Network"),
        });
        println!("current channel {} ", alice.channel);
        println!("Active viewers (strong count): {}", Rc::strong_count(&alice));

        println!("--> Bob enters the room and watches with Alice.");
        bob = Rc::clone(&alice);
        
        println!("Active viewers (strong count): {}", Rc::strong_count(&alice));
        
        println!("--> Alice leaves the room.");
    }

    println!("Active viewers (strong count): {}", Rc::strong_count(&bob));
    println!("Bob are we watching? : {}", bob.channel);
    
    println!("--> Bob leaves the house.");
}