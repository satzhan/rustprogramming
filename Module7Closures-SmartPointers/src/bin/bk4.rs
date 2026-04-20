use std::rc::Rc;
use std::cell::RefCell;

struct Tv {
    channel: String,
}

fn main() {
    let alice = Rc::new(RefCell::new(Tv {
        channel: String::from("Sports Network"),
    }));

    let bob = Rc::clone(&alice);

    println!("Alice is watching: {}", alice.borrow().channel);
    println!("Bob is watching: {}", bob.borrow().channel);

    {
        let mut remote = bob.borrow_mut();
        remote.channel = String::from("News Network");
    }

    println!("Alice is now watching: {}", alice.borrow().channel);
    println!("Bob is now watching: {}", bob.borrow().channel);
}