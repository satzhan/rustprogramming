use std::sync::atomic::{AtomicI32, Ordering};
use rand::Rng;
use std::sync::Arc;
use std::thread;

struct Account {
    balance: AtomicI32,
}

impl Account {
    fn new(initial_balance: i32) -> Self {
        Account {
            balance: AtomicI32::new(initial_balance),
        }
    }

    fn deposit(&self, amount: i32) -> bool {
        let mut rng = rand::thread_rng();

        // Simulating a failure 10% of the time.
        let current_balance = self.balance.load(Ordering::SeqCst);
        // if rng.gen_range(0..100) < 10 {
        //     return false;
        // }

        self.balance.store(current_balance + amount, Ordering::SeqCst);
        true
    }

    fn withdraw(&self, amount: i32) -> bool {
        let current_balance = self.balance.load(Ordering::SeqCst);

        if current_balance < amount {
            return false;
        }

        self.balance.store(current_balance - amount, Ordering::SeqCst);
        true
    }
}

fn transfer(source: &Account, destination: &Account, amount: i32) -> bool {
    if source.withdraw(amount) {
        if destination.deposit(amount) {
            true
        } else {
            println!("Deposit to destination failed! Rolling back...");
            source.deposit(amount);  // Revert the withdrawal.
            false
        }
    } else {
        println!("Withdrawal from source failed!");
        false
    }
}

fn main() {
    let n = std::thread::available_parallelism().unwrap().get();
    println!("Suggested worker count: {}", n);
    const N:i32 = 100000;
    const M:i32 = 10000;
    let source = Arc::new(Account::new(N));
    let destination = Arc::new(Account::new(N));

    let mut handles = vec![];

    for _ in 0..M {
        let source_clone = source.clone();
        let destination_clone = destination.clone();

        let handle = thread::spawn(move || {
            if !transfer(&source_clone, &destination_clone, N / M) {
                println!("Transfer failed!");
            }
        });

        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }
    
    println!("Final balance of source: {}", source.balance.load(Ordering::SeqCst));
    println!("Final balance of destination: {}", destination.balance.load(Ordering::SeqCst));
}