use std::collections::VecDeque;

struct RecentCounter {
    pings: VecDeque<i32>,
}

impl RecentCounter {
    fn new() -> Self {
        RecentCounter {
            pings: VecDeque::new(),
        }
    }
    fn ping(&mut self, t: i32) -> i32 {
        self.pings.push_back(t);
        while let Some(&front) = self.pings.front() {
            if front < t - 3000 {
                self.pings.pop_front();
            } else {
                break; 
            }
        }
        self.pings.len() as i32
    }
}

fn main() { // leetcode 933
    let mut counter = RecentCounter::new();

    let pings = [1, 100, 3001, 3002];

    for t in pings {
        let count = counter.ping(t);
        println!(
            "ping({:>4}) -> {} recent. Queue: {:?}",
            t, count, counter.pings
        );
    }
}