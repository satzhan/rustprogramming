struct BankAccount {
    balance: f64, // No RefCell needed
}

fn main() {
    let mut account = BankAccount { balance: 1000.0 };

    // You can't have two owners, but you can pass the account around
    deposit(&mut account, 500.0);
    withdraw(&mut account, 200.0);
}

fn deposit(account: &mut BankAccount, amount: f64) {
    account.balance += amount;
}
fn withdraw(account: &mut BankAccount, amount: f64) {
    account.balance -= amount;
}