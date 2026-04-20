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

    let mut remote = bob.borrow_mut();
    remote.channel = String::from("Movie Channel");
    
    drop(remote);

    let alice_view = alice.borrow(); 
    let bob_view = bob.borrow();     
    let guest_view = alice.borrow(); 

    println!("Alice sees: {}", alice_view.channel);
    println!("Bob sees:   {}", bob_view.channel);
    println!("Guest sees: {}", guest_view.channel);
}