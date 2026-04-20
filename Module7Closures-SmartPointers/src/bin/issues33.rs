use std::rc::Rc;
use std::cell::RefCell;

#[derive(Debug)]
struct Node {
    value: i32,
    next: Option<Rc<RefCell<Node>>>,
}

// Implement Drop so we can watch the cascade
impl Drop for Node {
    fn drop(&mut self) {
        println!("Destroying Node {}", self.value);
    }
}

fn main() {
    println!("--- 1. Creating independent nodes (C++ style) ---");
    // We create them all up front. Notice they do NOT need to be `mut`!
    // RefCell handles the mutability for us later.
    let root  = Rc::new(RefCell::new(Node { value: 0, next: None }));
    let node1 = Rc::new(RefCell::new(Node { value: 1, next: None }));
    let node2 = Rc::new(RefCell::new(Node { value: 2, next: None }));
    let node3 = Rc::new(RefCell::new(Node { value: 3, next: None }));

    println!("--- 2. Linking nodes arbitrarily ---");
    // We can link them in ANY order we want. 
    // We use `borrow_mut()` to access the inside, and `Rc::clone` to copy the pointer.
    
    // Let's link Root -> Node 1
    root.borrow_mut().next = Some(Rc::clone(&node1));
    
    // Let's link Node 2 -> Node 3 (Completely disconnected from Root for a moment!)
    node2.borrow_mut().next = Some(Rc::clone(&node3));
    
    // Now let's bridge the gap: Node 1 -> Node 2
    node1.borrow_mut().next = Some(Rc::clone(&node2));

    println!("List is fully linked!");

    println!("\n--- 3. Arming the Cascade ---");
    // Right now, Node 1 has an Rc count of 2 (owned by Root, AND owned by the `node1` variable).
    // If we want dropping `root` to cleanly cascade, we must relinquish our local handles
    // so that the List is the *only* thing keeping them alive.
    drop(node1);
    drop(node2);
    drop(node3); // we are manually dropping everything
    // and then root?
    
    println!("Local handles dropped. Root is now the sole owner of the chain.");

    println!("\n--- 4. Deleting Root ---");
    // This will now trigger the exact same beautiful cascade as Box!
    drop(root);

    println!("--- Program finished ---");
}