use std::rc::Rc;
use std::cell::RefCell;

struct Tv {
    channel: String,
}

struct Viewer {
    tv_connection: Rc<RefCell<Tv>>,
}

struct RemoteControl {
    tv_connection: Rc<RefCell<Tv>>,
}

fn main() {
    let shared_tv = Rc::new(RefCell::new(Tv {
        channel: String::from("Sports Network"),
    }));

    let alice = Viewer {
        tv_connection: Rc::clone(&shared_tv),
    };

    let bob = RemoteControl {
        tv_connection: Rc::clone(&shared_tv),
    };

    println!("Alice initially sees: {}", alice.tv_connection.borrow().channel);

    let mut bobs_access = bob.tv_connection.borrow_mut();
    bobs_access.channel = String::from("News Network");

    drop(bobs_access);

    println!("Alice now sees: {}", alice.tv_connection.borrow().channel);
}