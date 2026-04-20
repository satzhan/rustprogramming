use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

fn main() {
    // 1. We start with a bank account holding $100
    let bank_balance = Arc::new(Mutex::new(100));

    // Clone for Thread 1
    let balance_t1 = Arc::clone(&bank_balance);
    let handle1 = thread::spawn(move || {
        let has_funds = {
            let balance = balance_t1.lock().unwrap();
            println!("Thread 1: Checking balance... (${})", *balance);
            *balance >= 80 // We want to withdraw $80
        }; 
        thread::sleep(Duration::from_millis(50));

        // --- PHASE 2: USE ---
        if has_funds {
            println!("Thread 1: I have funds! Withdrawing $80...");
            let mut balance = balance_t1.lock().unwrap();
            *balance -= 80;
            println!("Thread 1: Withdrawal complete. Remaining: ${}", *balance);
        }
    });

    // Clone for Thread 2
    let balance_t2 = Arc::clone(&bank_balance);
    let handle2 = thread::spawn(move || {
        // We sleep -
        // --- PHASE 1: CHECK ---
        let has_funds = {
            let balance = balance_t2.lock().unwrap();
            println!("Thread 2: Checking balance... (${})", *balance);
            *balance >= 50 // We want to withdraw $50
        }; // Lock released!

        // --- PHASE 2: USE ---
        if has_funds {
            println!("Thread 2: I have funds! Withdrawing $50...");
            let mut balance = balance_t2.lock().unwrap();
            *balance -= 50;
            println!("Thread 2: Withdrawal complete. Remaining: ${}", *balance);
        }
    });

    handle1.join().unwrap();
    handle2.join().unwrap();

    println!("Final Bank Balance: ${}", *bank_balance.lock().unwrap());
}