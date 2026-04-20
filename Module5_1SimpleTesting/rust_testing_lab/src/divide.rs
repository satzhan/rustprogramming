#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn division_works() {
        assert_eq!(divide(10,2), 5);
    }

    #[test]
    #[should_panic]
    fn division_by_zero_panics() {
        divide(10, 0);
    }
}
pub fn divide(a: i32, b: i32) -> i32 {
    if b == 0 {
        panic!("cannot divide by zero");
    }
    a / b
}

