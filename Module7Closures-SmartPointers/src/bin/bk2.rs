#[derive(Debug)]
struct Node {
    value: i32,
    next: Option<Box<Node>>,
}

fn main() {
    let node3 = Node {
        value: 30,
        next: None,
    };

    let node2 = Node {
        value: 20,
        next: Some(Box::new(node3)),
    };

    let node1 = Node {
        value: 10,
        next: Some(Box::new(node2)),
    };

    println!("{:#?}", node1);
}