#[derive(Debug)]
struct BoxedValue<T> {
    value: T,
}

impl<T> BoxedValue<T> {
    fn new(value: T) -> Self {
        Self { value }
    }

    fn get_ref(&self) -> &T {
        &self.value
    }
}

fn main() {
    let a = BoxedValue::new(42);
    let b = BoxedValue::new("hello");

    println!("a = {:?}", a);
    println!("a value = {}", a.get_ref());

    println!("b = {:?}", b);
    println!("b value = {}", b.get_ref());
}