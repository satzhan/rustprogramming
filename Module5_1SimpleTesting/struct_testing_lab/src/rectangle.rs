#[derive(Debug, PartialEq)]
pub struct Rectangle {
    width: u32,
    height: u32,
}

impl Rectangle {
    pub fn new(width: u32, height: u32) -> Self {
        Rectangle { width, height }
    }

    pub fn area(&self) -> u32 {
        self.width * self.height
    }

    pub fn is_square(&self) -> bool {
        self.width == self.height
    }
    pub fn can_hold(&self, other: &Rectangle) -> bool {
        self.width > other.width && self.height > other.height
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rectangle_creation() {
        let rect = Rectangle::new(5, 10);
        assert_eq!(rect.width, 5);
        assert_eq!(rect.height, 10);
    }

    #[test]
    fn test_area() {
        let rect = Rectangle::new(5, 10);
        assert_eq!(rect.area(), 50);
    }

    #[test]
    fn test_is_square() {
        let square = Rectangle::new(5, 5);
        let rectangle = Rectangle::new(5, 10);
        assert!(square.is_square());
        assert!(!rectangle.is_square());
    }
    #[test]
    fn test_zero_width() {
        let rect = Rectangle::new(0, 10);
        assert_eq!(rect.area(), 0);
    }

    #[test]
    fn test_zero_height() {
        let rect = Rectangle::new(10, 0);
        assert_eq!(rect.area(), 0);
    }

    #[test]
    fn test_equality() {
        let rect1 = Rectangle::new(5, 10);
        let rect2 = Rectangle::new(5, 10);
        let rect3 = Rectangle::new(10, 5);
        assert_eq!(rect1, rect2);
        assert_ne!(rect1, rect3);
    }
    #[test]
    fn test_can_hold() {
        let larger = Rectangle::new(8, 7);
        let smaller = Rectangle::new(5, 1);
        assert!(larger.can_hold(&smaller));
        assert!(!smaller.can_hold(&larger));
    }
}