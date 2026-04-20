#[derive(Debug, PartialEq)]
pub enum WithdrawError {
    NonPositiveAmount, 
    InsuffientFunds { available: f64, requested: f64 },
}

pub struct BankAccount {
    balance: f64,
}

impl BankAccount {
    pub fn new() -> Self { Self { balance: 0.0 }}

    pub fn deposit(&mut self, amount: f64) -> Result<(), &'static str> {
        if amount <= 0.0 { return Err("amount must be positive"); }
        self.balance += amount;
        Ok(())
    }

    pub fn withdraw(&mut self, amount: f64) -> Result<(), WithdrawError> {
        if amount <= 0.0 {
            return Err(WithdrawError::NonPositiveAmount);
        }
        if amount > self.balance {
            return Err(WithdrawError::InsuffientFunds {
                available: self.balance,
                requested: amount,
            });
        }
        self.balance -= amount;
        Ok(())
    }

    pub fn balance(&self) -> f64 { self.balance }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn withdraw_ok() {
        let mut acct = BankAccount::new();
        acct.deposit(100.0).unwrap();
        assert_eq!(acct.withdraw(40.0), Ok(()));
        assert_eq!(acct.balance(), 60.0);
    }

    #[test]
    fn withdraw_too_much_is_err() {
        let mut acct = BankAccount::new();
        acct.deposit(10.0).unwrap();

        let err = acct.withdraw(50.0).unwrap_err();
        assert_eq!(
            err,
            WithdrawError::InsuffientFunds { available: 10.0, requested: 50.0 }
        );
    }
}