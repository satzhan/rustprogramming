#[derive(Debug)]
struct Node<'a> {
    value: i32,
    next: Option<&'a mut Node<'a>>, // 
} // assign lifetime explicitly because it's a pointer
// we can't live longer than the data we are pointing to
// we that data is deleted and we live longer
// then this pointer becomes dangling

fn main() {
    // 1. Create the nodes as standalone variables on the stack
    let mut node_a = Node { value: 1, next: None };
    let mut node_b = Node { value: 2, next: None };
    let mut node_c = Node { value: 3, next: None };

    // 2. Link them forward! (Mutating as we go)
    node_b.next = Some(&mut node_c);
    node_a.next = Some(&mut node_b);

    println!("Root: {:?}", node_a);
}

// can't return node_a from a function, 