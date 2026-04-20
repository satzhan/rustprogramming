use std::sync::{Arc, Barrier, Mutex};
use std::thread;
use std::time::Duration;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Side {
    Left,
    Right,
}

impl Side {
    fn switch(self) -> Self {
        match self {
            Side::Left => Side::Right,
            Side::Right => Side::Left,
        }
    }
}

#[derive(Debug)]
struct Hallway {
    alice: Side,
    bob: Side,
}

fn main() {
    let hallway = Arc::new(Mutex::new(Hallway {
        alice: Side::Left,
        bob: Side::Left,
    }));

    // Force both threads to act in lockstep so the livelock is easy to see.
    let barrier = Arc::new(Barrier::new(2));

    let alice = spawn_person("Alice", true, Arc::clone(&hallway), Arc::clone(&barrier));
    let bob = spawn_person("Bob", false, Arc::clone(&hallway), Arc::clone(&barrier));

    alice.join().unwrap();
    bob.join().unwrap();

    let final_state = hallway.lock().unwrap();
    println!("\nFinal hallway state: {:?}", *final_state);
    println!("Nobody passed. They stayed active, but made no progress.");
}

fn spawn_person(
    name: &'static str,
    is_alice: bool,
    hallway: Arc<Mutex<Hallway>>,
    barrier: Arc<Barrier>,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        for round in 1..=8 {
            // Phase 1: observe current state
            let (my_side, other_side) = {
                let h = hallway.lock().unwrap();
                if is_alice {
                    (h.alice, h.bob)
                } else {
                    (h.bob, h.alice)
                }
            };

            println!(
                "[{name}] round {round}: I am on {:?}, other is on {:?}",
                my_side, other_side
            );

            // If both are on the same side, be "polite" and switch.
            let should_switch = my_side == other_side;

            if should_switch {
                println!("[{name}] round {round}: Oops, same side. I'll switch.");
            } else {
                println!("[{name}] round {round}: Path is clear, I can pass.");
            }

            // Wait so both threads decide based on the same snapshot.
            barrier.wait();

            // Phase 2: apply decision
            {
                let mut h = hallway.lock().unwrap();
                if should_switch {
                    if is_alice {
                        h.alice = h.alice.switch();
                    } else {
                        h.bob = h.bob.switch();
                    }
                }
            }

            // Wait so both finish updating before next round.
            barrier.wait();

            thread::sleep(Duration::from_millis(150));
        }

        println!("[{name}] gave up after too many polite retries.");
    })
}