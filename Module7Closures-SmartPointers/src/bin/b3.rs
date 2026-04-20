use std::cell::RefCell;

fn main() {
    
    let data = RefCell::new(10);
    
    // Borrow the data immutably
    let data_ref = data.borrow();
    println!("Data: {}", data_ref);

    // Drop the immutable borrow so we can borrow mutably
    drop(data_ref);

    println!("Data: {:?}", data);

    // Borrow the data mutably
    let mut data_mut = data.borrow_mut();
    *data_mut += 1;
    println!("Data: {}", data_mut);
}