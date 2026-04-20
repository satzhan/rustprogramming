type NodeId = usize;

#[derive(Debug)]
struct Node {
    value: String,
    edges: Vec<NodeId>,
}

#[derive(Debug)]
struct Graph {
    nodes: Vec<Option<Node>>,
}

impl Graph {
    fn new() -> Self {
        Graph { nodes: Vec::new() }
    }

    fn add_node(&mut self, value: &str) -> NodeId {
        let id = self.nodes.len();
        self.nodes.push(Some(Node {
            value: value.to_string(),
            edges: Vec::new(),
        }));
        id
    }

    fn add_edge(&mut self, from: NodeId, to: NodeId) {
        if let Some(node) = self.nodes.get_mut(from).and_then(Option::as_mut) {
            node.edges.push(to);
        }
    }

    fn remove_node(&mut self, id: NodeId) {
        // 1. Remove the node itself
        if let Some(slot) = self.nodes.get_mut(id) {
            *slot = None;
        }

        // 2. Remove all incoming edges to that node
        for slot in &mut self.nodes {
            if let Some(node) = slot.as_mut() {
                node.edges.retain(|&target| target != id);
            }
        }
    }

    fn print_edge_target(&self, from: NodeId, edge_index: usize) {
        let Some(from_node) = self.nodes.get(from).and_then(Option::as_ref) else {
            println!("Source node {from} does not exist");
            return;
        };

        let Some(&target_id) = from_node.edges.get(edge_index) else {
            println!("Node {} has no edge at index {}", from, edge_index);
            return;
        };

        match self.nodes.get(target_id).and_then(Option::as_ref) {
            Some(target) => {
                println!("From {} -> found target safely: {}", from_node.value, target.value);
            }
            None => {
                println!("From {} -> target node no longer exists", from_node.value);
            }
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
    graph.print_edge_target(a, 0);
    graph.print_edge_target(b, 0);

    graph.remove_node(c);

    println!("\nAfter removal of C:");
    graph.print_edge_target(a, 0);
    graph.print_edge_target(b, 0);
}