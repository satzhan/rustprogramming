fn process_vector<F>(vec: Vec<i32>, f: F) -> Vec<i32>
where
    F: Fn(i32) -> i32,
{
    let mut result = Vec::new();
    for x in vec {
        result.push(f(x)); // Apply the closure
    }
    result
}

fn main() {
    let numbers = vec![1, 2, 3, 1, 5, 9];

    let doubled = process_vector(numbers.clone(), |x| {
        x * 2
    });

    let replaced = process_vector(numbers, |x| {
        if x > 2 { 0 } else { x }
    });

    println!("Doubled: {:?}", doubled);
    println!("Replaced: {:?}", replaced);
}