use std::sync::{Arc, Condvar, Mutex};

struct Foo {
    state: Mutex<u8>,
    cvar: Condvar,
}

impl Foo {
    fn new() -> Self {
        Foo {
            state: Mutex::new(1),
            cvar: Condvar::new(),
        }
    }

    fn first<F>(&self, print_first: F)
    where
        F: FnOnce(),
    {
        let mut step = self.state.lock().unwrap();

        print_first();

        *step = 2;
        self.cvar.notify_all();
    }

    fn second<F>(&self, print_second: F)
    where
        F: FnOnce(),
    {
        let mut step = self.state.lock().unwrap();

        while *step != 2 {
            step = self.cvar.wait(step).unwrap();
        }

        print_second();

        *step = 3;
        self.cvar.notify_all();
    }

    fn third<F>(&self, print_third: F)
    where
        F: FnOnce(),
    {
        let mut step = self.state.lock().unwrap();

        while *step != 3 {
            step = self.cvar.wait(step).unwrap();
        }

        print_third();
    }
}

use std::thread;

fn main() {
    let foo = Arc::new(Foo::new());

    let t3 = {
        let foo = Arc::clone(&foo);
        thread::spawn(move || {
            foo.third(|| println!("third"));
        })
    };

    let t2 = {
        let foo = Arc::clone(&foo);
        thread::spawn(move || {
            foo.second(|| println!("second"));
        })
    };

    let t1 = {
        let foo = Arc::clone(&foo);
        thread::spawn(move || {
            foo.first(|| println!("first"));
        })
    };

    t1.join().unwrap();
    t2.join().unwrap();
    t3.join().unwrap();

    println!("End");
}