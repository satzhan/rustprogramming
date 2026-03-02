#[derive(Debug)]
enum PaymentMethod {
    Cash,
    CreditCard,
    DebitCard,
    PayPal,
    BankTransfer,
}

fn main() {
    let cash = PaymentMethod::Cash;
    let credit = PaymentMethod::CreditCard;

    println!("{:?}, {:?}", cash, credit);
}