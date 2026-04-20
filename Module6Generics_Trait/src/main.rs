// Run this to see the baseline problem generics solve.
fn largest_int(list: &[i32]) -> i32 {
    let mut largest = list[0];
    for &item in list.iter() {
        if item > largest {
            largest = item;
        }
    }
    largest
}

fn largest_float(list: &[f32]) -> f32 {
    let mut largest = list[0];
    for &item in list.iter() {
        if item > largest {
            largest = item;
        }
    }
    largest
}

fn largest_char(list: &[char]) -> char {
    let mut largest = list[0];
    for &item in list.iter() {
        if item > largest {
            largest = item;
        }
    }
    largest
}

fn main() {
    println!("Largest int: {}", largest_int(&[1, 2, 5]));
    println!("Largest float: {}", largest_float(&[1.5, 2.6, 5.9]));
    println!("Largest char: {}", largest_char(&['A', 'B', 'C']));
}