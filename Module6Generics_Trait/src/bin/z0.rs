fn largest_i32(list: &[i32]) -> i32 {
    let mut largest = list[0];
    for &item in list.iter() {
        if item > largest {
            largest = item;
        }
    }
    largest
}

fn largest_f32(list: &[f32]) -> f32 {
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
    let nums = [3, 9, 2, 7];
    let floats = [1.2, 8.4, 3.3];
    let letters = ['a', 'z', 'm'];

    println!("largest_i32  = {}", largest_i32(&nums));
    println!("largest_f32  = {}", largest_f32(&floats));
    println!("largest_char = {}", largest_char(&letters));
}