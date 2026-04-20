use std::cell::RefCell;
use std::rc::{Rc, Weak};

type NodeId = usize;

#[derive(Debug)]
struct Node {
    value: String,
    edges: RefCell<Vec<Weak<Node>>>,
}

#[derive(Debug)]
struct Graph {
    nodes: Vec<Option<Rc<Node>>>,
}

impl Graph {
    fn new() -> Self {
        Self { nodes: Vec::new() }
    }

    fn add_node(&mut self, value: &str) -> NodeId {
        let id = self.nodes.len();
        let node = Rc::new(Node {
            value: value.to_string(),
            edges: RefCell::new(Vec::new()),
        });
        self.nodes.push(Some(node));
        id
    }

    fn add_edge(&self, from: NodeId, to: NodeId) {
        let Some(from_rc) = self.nodes.get(from).and_then(Option::as_ref) else {
            println!("from-node {from} does not exist");
            return;
        };

        let Some(to_rc) = self.nodes.get(to).and_then(Option::as_ref) else {
            println!("to-node {to} does not exist");
            return;
        };

        from_rc.edges.borrow_mut().push(Rc::downgrade(to_rc));
    }

    fn remove_node(&mut self, id: NodeId) {
        if let Some(slot) = self.nodes.get_mut(id) {
            *slot = None;
        }
    }

    fn print_first_edge_target(&self, from: NodeId) {
        let Some(from_rc) = self.nodes.get(from).and_then(Option::as_ref) else {
            println!("Source node {from} does not exist");
            return;
        };

        let edges = from_rc.edges.borrow();

        let Some(weak_target) = edges.first() else {
            println!("{} has no outgoing edges", from_rc.value);
            return;
        };

        match weak_target.upgrade() {
            Some(target_rc) => {
                println!("{} -> {}", from_rc.value, target_rc.value);
            }
            None => {
                println!("{} -> dead edge", from_rc.value);
            }
        }
    }

    fn compact_dead_edges(&self, id: NodeId) {
        let Some(node_rc) = self.nodes.get(id).and_then(Option::as_ref) else {
            return;
        };

        node_rc
            .edges
            .borrow_mut()
            .retain(|weak| weak.upgrade().is_some());
    }

    fn edge_count(&self, id: NodeId) -> usize {
        self.nodes
            .get(id)
            .and_then(Option::as_ref)
            .map(|node| node.edges.borrow().len())
            .unwrap_or(0)
    }

    fn print_node_value(&self, id: NodeId) {
        match self.nodes.get(id).and_then(Option::as_ref) {
            Some(node) => println!("Node {id}: {}", node.value),
            None => println!("Node {id} does not exist"),
        }
    }
}

fn main() {
    let mut graph = Graph::new();

    let a = graph.add_node("Node A");
    let b = graph.add_node("Node B");
    let c = graph.add_node("Node C");

    graph.add_edge(a, c);
    graph.add_edge(b, c);

    println!("Before removal:");
    graph.print_node_value(a);
    graph.print_node_value(b);
    graph.print_node_value(c);
    graph.print_first_edge_target(a);
    graph.print_first_edge_target(b);
    println!("A edge count: {}", graph.edge_count(a));
    println!("B edge count: {}", graph.edge_count(b));

    graph.remove_node(c);

    println!("\nAfter removing C from graph:");
    graph.print_first_edge_target(a);
    graph.print_first_edge_target(b);
    println!("A edge count: {}", graph.edge_count(a));
    println!("B edge count: {}", graph.edge_count(b));

    graph.compact_dead_edges(a);
    graph.compact_dead_edges(b);

    println!("\nAfter compacting dead edges:");
    graph.print_first_edge_target(a);
    graph.print_first_edge_target(b);
    println!("A edge count: {}", graph.edge_count(a));
    println!("B edge count: {}", graph.edge_count(b));
}