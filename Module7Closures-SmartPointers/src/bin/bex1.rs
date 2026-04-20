fn main() {
    let var = 100;
    let box_default = Box::new(var); // heap
    println!("{}", box_default);  // Output: 100
}