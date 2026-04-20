#[derive(Debug)]
struct Node {
    value: i32,
    next: Option<Box<Node>>,
}

// We implement Drop to prove exactly when memory is freed
impl Drop for Node {
    fn drop(&mut self) {
        println!("Destroying Node {}", self.value);
    }
}

fn main() {
    println!("--- 1. Creating independent nodes ---");
    // We must make them `mut` so we can change their `next` fields later.
    let mut root  = Box::new(Node { value: 0, next: None });
    let mut node1 = Box::new(Node { value: 1, next: None });
    let mut node2 = Box::new(Node { value: 2, next: None });
    let mut node3 = Box::new(Node { value: 3, next: None });

    println!("--- 2. Linking nodes together ---");
    // We link them backwards. Why? Because we are TRANSFERRING OWNERSHIP.
    
    // node3 is MOVED into node2's next field. 
    // The variable `node3` is now empty and can no longer be used!
    node2.next = Some(node3); 
    
    // node2 is MOVED into node1.
    node1.next = Some(node2); 
    
    // node1 is MOVED into the root.
    root.next = Some(node1);  

    println!("List is fully linked under Root!");

    // err
    // println!("Let's look at node3: {:?}", node3);
    // Error: "borrow of moved value: `node3`"

    println!("\n--- 3. Deleting Root ---");
    // We manually delete the Root. Because Root now owns node1, 
    // which owns node2, which owns node3... they will all fall like dominoes.
    drop(root); 

    println!("--- Program finished ---");
}