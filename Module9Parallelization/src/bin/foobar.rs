use std::sync::{Arc, Condvar, Mutex};
use std::thread;

struct FooBar {
    n: usize,
    foo_turn: Mutex<bool>,
    cv: Condvar,
}

impl FooBar {
    fn new(n: usize) -> Self {
        FooBar {
            n,
            foo_turn: Mutex::new(true), // true => foo's turn, false => bar's turn
            cv: Condvar::new(),
        }
    }

    fn foo<F>(&self, print_foo: F)
    where
        F: Fn(),
    {
        for _ in 0..self.n {
            let mut foo_turn = self.foo_turn.lock().unwrap();

            while !*foo_turn {
                foo_turn = self.cv.wait(foo_turn).unwrap();
            }

            print_foo();

            *foo_turn = false;
            self.cv.notify_one();
        }
    }

    fn bar<F>(&self, print_bar: F)
    where
        F: Fn(),
    {
        for _ in 0..self.n {
            let mut foo_turn = self.foo_turn.lock().unwrap();
            while *foo_turn {
                foo_turn = self.cv.wait(foo_turn).unwrap();
            }
            print_bar();
            *foo_turn = true;
            self.cv.notify_one();
        }
    }
}

fn main() {
    let foobar = Arc::new(FooBar::new(10));

    let t1 = {
        let foobar = Arc::clone(&foobar);
        thread::spawn(move || {
            foobar.foo(|| print!("foo"));
        })
    };

    let t2 = {
        let foobar = Arc::clone(&foobar);
        thread::spawn(move || {
            foobar.bar(|| print!("bar "));
        })
    };

    t1.join().unwrap();
    t2.join().unwrap();

    println!();
}