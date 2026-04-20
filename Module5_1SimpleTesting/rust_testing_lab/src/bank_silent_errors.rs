pub struct BankAccount {
    balance: f64,
}

impl BankAccount {
    pub fn new() -> Self { Self { balance: 0.0 } }

    pub fn deposit(&mut self, amount: f64) {
        if amount > 0.0 {
            self.balance += amount;
        }
    }

    pub fn withdraw(&mut self, amount: f64) {
        if amount > 0.0 && amount <= self.balance { 
            self.balance -= amount;
        }
        // else: do nothing (silent)
    }
    pub fn balance(&self) -> f64 { self.balance }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn withdraw_decreases_balance() {
        let mut acct = BankAccount::new();
        acct.deposit(100.0);
        acct.withdraw(30.0);
        assert_eq!(acct.balance(), 70.0);
    }

    #[test]
    fn withdrawing_too_much_does_nothing() {
        let mut acct = BankAccount::new();
        acct.deposit(50.0);
        acct.withdraw(70.0);
        assert_eq!(acct.balance(), 50.0);
    }
}