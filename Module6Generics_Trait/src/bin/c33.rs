use std::cmp::PartialOrd;

fn largest<T: PartialOrd + Copy>(list: &[T]) -> T {
    let mut largest = list[0];
    for &item in list.iter() {
        if item > largest {
            largest = item;
        }
    }
    largest
}

fn main() {
    let nums = [3, 9, 2, 7];
    let floats = [1.2, 8.4, 3.3];
    let letters = ['a', 'z', 'm'];

    println!("largest num    = {}", largest(&nums));
    println!("largest float  = {}", largest(&floats));
    println!("largest char   = {}", largest(&letters));
}