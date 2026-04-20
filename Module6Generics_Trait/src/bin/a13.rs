#[derive(Debug)]
struct Stack<T> {
    items: Vec<T>,
}

impl<T> Stack<T> {
    fn new() -> Self {
        Self { items: Vec::new() }
    }

    fn push(&mut self, value: T) {
        self.items.push(value);
    }

    fn pop(&mut self) -> Option<T> {
        self.items.pop()
    }

    fn peek(&self) -> Option<&T> {
        self.items.last()
    }

    fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    fn len(&self) -> usize {
        self.items.len()
    }
}

fn main() {
    let mut numbers = Stack::new();
    numbers.push(10);
    numbers.push(20);
    numbers.push(30);

    println!("numbers = {:?}", numbers);
    println!("top     = {:?}", numbers.peek());
    println!("pop     = {:?}", numbers.pop());
    println!("after   = {:?}", numbers);
    println!("len     = {}", numbers.len());
    println!("empty   = {}", numbers.is_empty());

    let mut words = Stack::new();
    words.push(String::from("red"));
    words.push(String::from("blue"));

    println!("\nwords = {:?}", words);
    println!("top   = {:?}", words.peek());
    println!("pop   = {:?}", words.pop());
    println!("after = {:?}", words);
}