pub struct Rectangle {
    pub width: u32,
    pub height: u32,
}

impl Rectangle {
    pub fn area(&self) -> u32 {
        self.width * self.height
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rectangle_area() {
        let r = Rectangle { width: 3, height: 4};
        assert_eq!(r.area(), 12);
    }
}