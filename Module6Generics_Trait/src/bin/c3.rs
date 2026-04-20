fn largest<T: PartialOrd + Copy>(list: &[T]) -> T { 
    let mut largest = list[0]; // copy of T
    for &item in list.iter() {
        if item > largest { // comparison T > T
            largest = item; 
        }
    }
    largest
}

fn main() {
    let number_list = vec![34, 50, 25, 100, 65];
    let result_num = largest(&number_list);
    println!("The largest number is {}", result_num);

    let char_list = vec!['y', 'm', 'a', 'q'];
    let result_char = largest(&char_list);
    println!("The largest char is {}", result_char);
}