#[warn(dead_code)]

#[derive(Debug)]
struct Node {
    value: i32,
    next: Option<Box<Node>>,
}

fn main() {
    let node3 = Node {
        value: 33,
        next: None,
    };

    let node2 = Node {
        value: 22,
        next: Some(Box::new(node3)),
    };

    let node1 = Node {
        value: 11,
        next: Some(Box::new(node2)),
    };

    println!("My linked list: {:#?}", node1);
}