use std::rc::Rc;
use std::cell::RefCell;

fn main() {
    let shared_data = Rc::new(RefCell::new(String::from("Hello")));

    let owner_two = Rc::clone(&shared_data);
    let owner_three = Rc::clone(&shared_data);

    *owner_two.borrow_mut() = String::from("Hello, World!");

    println!("Owner three sees: {}", owner_three.borrow());
}