use std::rc::Rc;
use std::cell::RefCell;

fn main() {
    #[derive(Debug)]
    struct BankAccount {
        balance: RefCell<f64>,
    }
    
    impl BankAccount {
        fn new(initial_balance: f64) -> Rc<Self> {
            Rc::new(BankAccount {
                balance: RefCell::new(initial_balance),
            })
        }
    
        fn deposit(&self, amount: f64) {
            let mut balance = self.balance.borrow_mut();
            *balance += amount;
            println!("Deposited ${:.2}, new balance: ${:.2}", amount, *balance);
        }
    
        fn withdraw(&self, amount: f64) {
            let mut balance = self.balance.borrow_mut();
            if *balance >= amount {
                *balance -= amount;
                println!("Withdrew ${:.2}, new balance: ${:.2}", amount, *balance);
            } else {
                println!("Insufficient funds. Current balance: ${:.2}", *balance);
            }
        }
    }
    
    let account = BankAccount::new(1000.0);
    let joint_account = Rc::clone(&account);

    account.deposit(500.0);
    joint_account.withdraw(200.0);
    account.withdraw(1500.0);
}