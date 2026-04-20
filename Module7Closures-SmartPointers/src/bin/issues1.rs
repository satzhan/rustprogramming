struct Node {
    id: String,
}

fn main() {
    // 1. The OS allocates memory for Node C
    let node_c = Node { id: String::from("C") };

    // 2. A and B borrow Node C (holding references to its memory address)
    let pointer_a = &node_c;
    let pointer_b = &node_c;

    // 3. We attempt to manually drop (free) the memory of Node C
    drop(node_c); 

    // 4. We attempt to read from the now-freed memory
    println!("Pointer A reads: {}", pointer_a.id);
}