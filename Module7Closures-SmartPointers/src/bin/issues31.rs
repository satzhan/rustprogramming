#[derive(Debug)]
struct Node {
    value: i32,
    next: Option<Box<Node>>, // Box means OWNERSHIP. No lifetimes needed!
}

// We implement Drop just to print a message when a node is destroyed
impl Drop for Node {
    fn drop(&mut self) {
        println!("Destroying Node {}", self.value);
    }
}

fn main() {
    println!("--- Building the list ---");
    
    // We build the list: A -> B -> C -> None
    // We construct it from the Tail (C) to the Head (A)
    let node_c = Box::new(Node { value: 3, next: None });
    let node_b = Box::new(Node { value: 2, next: Some(node_c) });
    
    // node_a is our Root. It owns the Box pointing to B.
    let node_a = Box::new(Node { value: 1, next: Some(node_b) });

    println!("List created successfully!\n");

    println!("--- Manually dropping Node A (The Root) ---");
    
    // We manually trigger the deletion of the Root node
    drop(node_a); 

    println!("--- Program finished ---");
}