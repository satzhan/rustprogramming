// Run this to see dynamic dispatch in action using the `dyn` keyword.
pub trait AreaInfo {
    fn get_area(&self) -> f64;
}

pub struct Rectangle {
    pub width: f64,
    pub height: f64,
}

impl AreaInfo for Rectangle {
    fn get_area(&self) -> f64 {
        self.width * self.height
    }
}

pub struct Circle {
    pub radius: f64,
}

impl AreaInfo for Circle {
    fn get_area(&self) -> f64 {
        self.radius * self.radius * 3.14
    }
}

fn main() {
    let rec = Rectangle { width: 5.0, height: 8.0 };
    let circle = Circle { radius: 5.0 };

    // We can store different types in the same Vector because 
    // we use a reference to the Trait they both implement.
    let shapes: Vec<&dyn AreaInfo> = vec![&rec, &circle];

    for (index, shape) in shapes.iter().enumerate() {
        println!("Area of shape {}: {}", index + 1, shape.get_area());
    }
}