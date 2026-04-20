mod rectangle;

use rectangle::Rectangle;

fn main() {
    let rect = Rectangle::new(5, 10);
    println!("Rectangle: {:?}", rect);
    println!("Area: {}", rect.area());
    println!("Is square? {}", rect.is_square());
}