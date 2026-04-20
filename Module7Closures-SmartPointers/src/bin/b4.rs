use std::rc::Rc;
use std::cell::RefCell;

fn main() {
    #[derive(Debug)]
    struct MyData {
        data: f64
    }

    let base: Rc<RefCell<MyData>> = Rc::new(RefCell::new(
        MyData {
            data: 70.00
        }
    ));

    println!("base: {:?}", base);
    // let ref1 = base.borrow();
    {
        let mut base_2 = base.borrow_mut();
        base_2.data -= 10.00;
        println!("base_2: {:?}", base_2);
    }
 
    println!("base: {:?}", base);
 
    let mut base_3 = base.borrow_mut();
    base_3.data += 30.00;
 
    println!("base: {:?}", base);
    println!("base_3: {:?}", base_3);
    drop(base_3);
    println!("base: {:?}", base);
}