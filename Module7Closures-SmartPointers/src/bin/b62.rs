use std::collections::HashMap;

struct Bank {
    accounts: HashMap<u32, f64>,
}

fn main() {
    let mut bank = Bank { accounts: HashMap::new() };
    bank.accounts.insert(1, 1000.0); // Create Account ID 1

    let my_account_id = 1;
    let joint_account_id = 1; // Both just hold the ID "1"

    // To use it, you ask the Bank to mutate the data for that ID
    if let Some(balance) = bank.accounts.get_mut(&my_account_id) {
        *balance += 500.0;
    }
}