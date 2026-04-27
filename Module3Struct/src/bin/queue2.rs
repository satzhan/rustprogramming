use std::collections::VecDeque;

fn main() { // more simple deque
    let mut queue: VecDeque<i32> = VecDeque::new();
    // Add numbers to the back
    for n in 1..=10 {
        queue.push_back(n);
        println!("Pushed {} to back. Queue: {:?}", n, queue);
    }
    println!();
    // Remove from the front
    while let Some(n) = queue.pop_front() {
        println!("Popped {} from front. Queue: {:?}", n, queue);
    }
}