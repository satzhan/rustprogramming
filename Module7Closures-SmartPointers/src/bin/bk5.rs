use std::rc::{Rc, Weak};
use std::cell::RefCell;

#[derive(Debug)]
struct Node {
    name: String,
    child: RefCell<Option<Rc<Node>>>,
    parent: RefCell<Weak<Node>>, 
}

fn main() {
    let parent = Rc::new(Node {
        name: String::from("Parent"),
        child: RefCell::new(None),
        parent: RefCell::new(Weak::new()),
    });

    println!("Parent strong: {}, weak: {}", Rc::strong_count(&parent), Rc::weak_count(&parent));

    let child = Rc::new(Node {
        name: String::from("Child"),
        child: RefCell::new(None),
        parent: RefCell::new(Rc::downgrade(&parent)), 
    });

    *parent.child.borrow_mut() = Some(Rc::clone(&child));

    println!("--- After Linking ---");
    println!("Parent strong: {}, weak: {}", Rc::strong_count(&parent), Rc::weak_count(&parent));
    println!("Child strong: {}, weak: {}", Rc::strong_count(&child), Rc::weak_count(&child));
}