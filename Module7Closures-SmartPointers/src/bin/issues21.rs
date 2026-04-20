use std::cell::RefCell;
use std::rc::{Rc, Weak};

#[derive(Debug)]
struct Node {
    value: String,
    edges: Vec<Weak<RefCell<Node>>>,
}

impl Node {
    fn new(value: &str) -> Rc<RefCell<Node>> {
        Rc::new(RefCell::new(Node {
            value: value.to_string(),
            edges: Vec::new(),
        }))
    }
}

fn main() {
    let node_a = Node::new("Node A");
    let node_b = Node::new("Node B");

    // C has one strong owner here: node_c
    let node_c = Node::new("Node C");

    // A and B only keep weak links to C
    node_a.borrow_mut().edges.push(Rc::downgrade(&node_c));
    node_b.borrow_mut().edges.push(Rc::downgrade(&node_c));

    println!("Before drop:");
    println!("strong_count(C) = {}", Rc::strong_count(&node_c)); // 1
    println!("weak_count(C)   = {}", Rc::weak_count(&node_c));   // 2

    // A can still reach C for now, because node_c still exists as a strong owner
    if let Some(c_from_a) = node_a.borrow().edges[0].upgrade() {
        println!("A sees: {}", c_from_a.borrow().value);
    } else {
        println!("A sees nothing");
    }

    // Drop the last strong owner
    drop(node_c);

    println!("\nAfter drop:");

    // Now A's weak edge can no longer be upgraded
    match node_a.borrow().edges[0].upgrade() {
        Some(c_from_a) => println!("A still sees: {}", c_from_a.borrow().value),
        None => println!("A can no longer access C"),
    }

    match node_b.borrow().edges[0].upgrade() {
        Some(c_from_b) => println!("B still sees: {}", c_from_b.borrow().value),
        None => println!("B can no longer access C"),
    }
}