use std::thread;
use std::time::Duration;

fn main() {
    println!("main: start");

    thread::scope(|s| {
        s.spawn(|| {
            for i in 1..=5 {
                println!("child thread: step {i}");
                thread::sleep(Duration::from_millis(120));
            }
            println!("child thread: done");
        });

        // Main thread is also working inside the same scope
        for i in 1..=3 {
            println!("main thread inside scope: step {i}");
            thread::sleep(Duration::from_millis(200));
        }

        println!("main thread: reached end of scope body");
    });

    println!("main: after scope");
}