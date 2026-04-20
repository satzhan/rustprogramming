pub struct BankAccount { 
    balance: f64, // private
}
impl BankAccount {
    pub fn new() -> Self {
        Self { balance: 0.0 }
    }
    pub fn deposit( &mut self, amount: f64) {
        self.balance += amount;
    }
    pub fn balance(&self) -> f64 {
        self.balance
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn deposit_increases_balance() {
        let mut acct = BankAccount::new();
        acct.deposit(50.0);
        assert_eq!(acct.balance(), 50.0);
    }
}