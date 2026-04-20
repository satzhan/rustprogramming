use std::rc::Rc;

fn main() {
    let num_in_heap = Rc::new(10);

    let _copy2_of_num = Rc::clone(&num_in_heap);
    let _copy3_of_num = Rc::clone(&num_in_heap);
    let _copy4_of_num = Rc::clone(&num_in_heap);

    println!("num in heap has: {} references", 
             Rc::strong_count(&num_in_heap));
}