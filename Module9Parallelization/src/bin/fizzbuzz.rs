use std::thread;
use std::sync::{Arc, Condvar, Mutex};

struct State {
    current: i32,
}

struct FizzBuzz {
    n: i32,
    state: Arc<(Mutex<State>, Condvar)>,
}

impl FizzBuzz {
    fn new(n: i32) -> Self {
        Self {
            n,
            state: Arc::new((Mutex::new(State { current: 1 }), Condvar::new())),
        }
    }

    // printFizz() outputs "fizz"
    fn fizz(&self, printFizz: impl Fn() + Send + 'static) {
        let (lock, cvar) = &*self.state;

        loop {
            let mut state = lock.lock().unwrap();

            while state.current <= self.n
                && !(state.current % 3 == 0 && state.current % 5 != 0)
            {
                state = cvar.wait(state).unwrap();
            }

            if state.current > self.n {
                cvar.notify_all();
                break;
            }

            printFizz();
            state.current += 1;
            cvar.notify_all();
        }
    }

    // printBuzz() outputs "buzz"
    fn buzz(&self, printBuzz: impl Fn() + Send + 'static) {
        let (lock, cvar) = &*self.state;

        loop {
            let mut state = lock.lock().unwrap();

            while state.current <= self.n
                && !(state.current % 5 == 0 && state.current % 3 != 0)
            {
                state = cvar.wait(state).unwrap();
            }

            if state.current > self.n {
                cvar.notify_all();
                break;
            }

            printBuzz();
            state.current += 1;
            cvar.notify_all();
        }
    }

    // printFizzBuzz() outputs "fizzbuzz"
    fn fizzbuzz(&self, printFizzBuzz: impl Fn() + Send + 'static) {
        let (lock, cvar) = &*self.state;

        loop {
            let mut state = lock.lock().unwrap();

            while state.current <= self.n
                && !(state.current % 15 == 0)
            {
                state = cvar.wait(state).unwrap();
            }

            if state.current > self.n {
                cvar.notify_all();
                break;
            }

            printFizzBuzz();
            state.current += 1;
            cvar.notify_all();
        }
    }

    // printNumber(x) outputs "x"
    fn number(&self, printNumber: impl Fn(i32) + Send + 'static) {
        let (lock, cvar) = &*self.state;

        loop {
            let mut state = lock.lock().unwrap();

            while state.current <= self.n
                && !(state.current % 3 != 0 && state.current % 5 != 0)
            {
                state = cvar.wait(state).unwrap();
            }

            if state.current > self.n {
                cvar.notify_all();
                break;
            }

            printNumber(state.current);
            state.current += 1;
            cvar.notify_all();
        }
    }
}

fn main() {
    let fb = Arc::new(FizzBuzz::new(20));

    let a = {
        let fb = Arc::clone(&fb);
        thread::spawn(move || {
            fb.fizz(|| print!("fizz "));
        })
    };

    let b = {
        let fb = Arc::clone(&fb);
        thread::spawn(move || {
            fb.buzz(|| print!("buzz "));
        })
    };

    let c = {
        let fb = Arc::clone(&fb);
        thread::spawn(move || {
            fb.fizzbuzz(|| print!("fizzbuzz "));
        })
    };

    let d = {
        let fb = Arc::clone(&fb);
        thread::spawn(move || {
            fb.number(|x| print!("{} ", x));
        })
    };

    a.join().unwrap();
    b.join().unwrap();
    c.join().unwrap();
    d.join().unwrap();

    println!();
}