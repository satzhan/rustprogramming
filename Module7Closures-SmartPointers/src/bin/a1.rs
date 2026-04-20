fn main() {
    let threshold = 10;

    // fn check_limit(value: i32) -> bool {
    //     value > threshold
    // }

    let check_limit = |value: i32| -> bool {
        value > threshold
    };

    println!("is 15 over limit? {}", check_limit(15));
    println!("is 5 over limit? {}", check_limit(5));

    let check_limit = |value: i32| value > threshold;
    
    println!("is 15 over limit? {}", check_limit(15));
    println!("is 5 over limit? {}", check_limit(5));

}