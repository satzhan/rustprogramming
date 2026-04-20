use std::collections::HashMap;

#[derive(Debug)]
struct Pet<T> {
    cats: T,
    dogs: T,
}

impl<T> Pet<T> {
    fn new(cats: T, dogs: T) -> Self {
        Self { cats, dogs }
    }
}

fn main() {
    // 1) Turbofish on a generic type constructor
    let numbers = Vec::<i32>::new();
    println!("empty numbers vec = {:?}", numbers);

    // 2) Turbofish on a generic struct constructor
    let pet_counts = Pet::<i64>::new(5, 10);
    println!("pet_counts = {:?}", pet_counts);

    let pet_names = Pet::<String>::new("Milo".into(), "Bolt".into());
    println!("pet_names = {:?}", pet_names);

    // 3) Turbofish on collect()
    let pairs = vec![("alice", 90), ("bob", 95)];

    let scores = pairs
        .into_iter()
        .map(|(name, score)| (name.to_string(), score))
        .collect::<HashMap<String, i32>>();

    println!("scores = {:?}", scores);
}