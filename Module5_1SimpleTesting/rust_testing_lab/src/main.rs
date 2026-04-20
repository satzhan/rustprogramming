fn add(a: i32, b: i32) -> i32 {
    a + b
}
fn is_even(n: i32) -> bool {
    n % 2 == 0
}
fn divide(a: i32, b: i32) -> i32 {
    if b == 0 {
        panic!("Cannot divide by zero");
    }
    a / b
}

fn main() {
    println!("2 + 2 = {}", add(2, 2));
    println!("{} panic?", divide(10,0));
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_add() {
        assert_eq!(add(2, 2), 4);
    }
    #[test]
    fn test_add_negative() {
        assert_eq!(add(-2, -2), -4);
    }

    #[test]
    fn test_add_zero() {
        assert_eq!(add(0, 0), 0);
    }
    #[test]
    fn test_is_even() {
        assert!(is_even(2));
        assert!(!is_even(3));
    }

    #[test]
    fn test_not_equal() {
        assert_ne!(add(2, 2), 5);
    }
    #[test]
    #[should_panic(expected = "Cannot divide by zero")]
    fn test_divide_by_zero() {
        divide(10, 0);
    }
    #[test]
    fn test_add_multiple() {
        let test_cases = vec![
            (1, 1, 2),
            (0, 0, 0),
            (-1, 1, 0),
            (100, -50, 50)
        ];
        
        for (a, b, expected) in test_cases {
            assert_eq!(add(a, b), expected, "Failed on input ({}, {})", a, b);
        }
    }
    #[test]
    fn test_is_even_verbose() {
        let number = 5;
        assert!(
            is_even(number),
            "Expected {} to be even, but it was odd",
            number
        );
    }
    #[test]
    fn test_complex_add() {
        assert_eq!(add(add(1, 2), add(3, 4)), 10);
    }

    #[test]
    fn test_even_arithmetic() {
        assert!(is_even(add(2, 2)));
        assert!(!is_even(add(2, 3)));
    }
}