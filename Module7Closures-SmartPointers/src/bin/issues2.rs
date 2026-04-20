use std::cell::RefCell;
use std::rc::Rc;

struct Node {
    value: String,
    edges: Vec<Rc<RefCell<Node>>>,
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
    
    // 1. Create Node C. Count is 1.
    let node_c = Node::new("Node C"); 

    // 2. Link A and B to C. Count becomes 3.
    node_a.borrow_mut().edges.push(Rc::clone(&node_c));
    node_b.borrow_mut().edges.push(Rc::clone(&node_c));
    
    println!("Count before drop: {}", Rc::strong_count(&node_a.borrow().edges[0])); // Output: 3

    // 3. DROP C
    // We are explicitly destroying the local variable `node_c`.
    drop(node_c);

    // If we uncommented the next line, the program would NOT compile:
    // println!("Trying to use node_c directly: {}", node_c.borrow().value);

    // 4. Look at the count now by peeking through Node A's edge!
    let a_reads = node_a.borrow();
    let target_node = a_reads.edges[0].borrow();
    
    // The count is 2! The memory is totally safe.
    println!("Count after drop: {}", Rc::strong_count(&a_reads.edges[0])); // Output: 2
    
    // We can still read the data!
    println!("Traversing from A... Found target safely: {}", target_node.value); // Output: Node C
}