fn main() {
    let mut violations = 0;
    let threshold = 10;

    let mut check_and_count = |value: i32| -> bool {
        if value > threshold {
            violations += 1;
            true
        } else {
            false
        }
    };


    println!("is it over 15? {}", check_and_count(15));
    println!("is it over 12? {}", check_and_count(12));

    //println!("Current violations: {}", violations); // err
    
    println!("is it over 12? {}", check_and_count(12));

    drop(check_and_count);
    println!("total violations: {}", violations);
}