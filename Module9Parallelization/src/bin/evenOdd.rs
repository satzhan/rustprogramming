use std::sync::{Arc, Condvar, Mutex};
use std::thread;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Turn {
    Zero,
    Odd,
    Even,
}

#[derive(Debug)]
struct State {
    next: i32,
    turn: Turn,
}

struct ZeroEvenOdd {
    n: i32,
    state: Mutex<State>,
    cv: Condvar,
}

impl ZeroEvenOdd {
    fn new(n: i32) -> Self {
        Self {
            n,
            state: Mutex::new(State {
                next: 1,
                turn: Turn::Zero,
            }),
            cv: Condvar::new(),
        }
    }

    fn zero<F>(&self, print_number: F)
    where
        F: Fn(i32),
    {
        for _ in 0..self.n as usize {
            let mut state = self.state.lock().unwrap();

            while state.turn != Turn::Zero {
                state = self.cv.wait(state).unwrap();
            }

            // Print 0
            print_number(0);

            // Decide whether odd or even should go next
            if state.next % 2 == 1 {
                state.turn = Turn::Odd;
            } else {
                state.turn = Turn::Even;
            }

            self.cv.notify_all();
        }
    }

    fn odd<F>(&self, print_number: F)
    where
        F: Fn(i32),
    {
        for _ in 0..((self.n + 1) / 2) as usize {
            let mut state = self.state.lock().unwrap();

            while state.turn != Turn::Odd {
                state = self.cv.wait(state).unwrap();
            }

            // Print current odd number
            print_number(state.next);
            state.next += 1;
            state.turn = Turn::Zero;

            self.cv.notify_all();
        }
    }

    fn even<F>(&self, print_number: F)
    where
        F: Fn(i32),
    {
        for _ in 0..(self.n / 2) as usize {
            let mut state = self.state.lock().unwrap();

            while state.turn != Turn::Even {
                state = self.cv.wait(state).unwrap();
            }

            // Print current even number
            print_number(state.next);
            state.next += 1;
            state.turn = Turn::Zero;

            self.cv.notify_all();
        }
    }
}

fn main() {
    let n = 5;
    let zeo = Arc::new(ZeroEvenOdd::new(n));

    let t_zero = {
        let zeo = Arc::clone(&zeo);
        thread::spawn(move || {
            zeo.zero(|x| print!("{}", x));
        })
    };

    let t_odd = {
        let zeo = Arc::clone(&zeo);
        thread::spawn(move || {
            zeo.odd(|x| print!("{}", x));
        })
    };

    let t_even = {
        let zeo = Arc::clone(&zeo);
        thread::spawn(move || {
            zeo.even(|x| print!("{}", x));
        })
    };

    t_zero.join().unwrap();
    t_odd.join().unwrap();
    t_even.join().unwrap();

    println!();
}