#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn even_checks() {
        assert!(is_even(4));
        assert!(!is_even(5));
        assert_ne!(is_even(3), true);
    }
}

pub fn is_even(x: i32) -> bool {
    x % 2 == 0
}