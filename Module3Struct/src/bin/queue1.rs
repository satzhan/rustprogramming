use std::collections::VecDeque;

fn main() { // simple deque
    let mut queue: VecDeque<String> = VecDeque::new();

    // Add to the back
    queue.push_back(String::from("Alice"));
    queue.push_back(String::from("Bob"));
    queue.push_back(String::from("Carol"));

    println!("Queue: {:?}", queue);

    // Remove from the front
    while let Some(person) = queue.pop_front() {
        println!("Serving: {}", person);
    }

    println!("Queue: {:?}", queue);
}
