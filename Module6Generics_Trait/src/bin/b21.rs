

#[derive(Debug)]
struct Rectangle {
    width: f64,
    height: f64,
}

#[derive(Debug)]
struct Circle {
    radius: f64,
}
struct Triangle {
    base: f64,
    height: f64,
}
trait AreaInfo {
    fn get_area(&self) -> f64;
}
impl AreaInfo for Rectangle {
    fn get_area(&self) -> f64 {
        self.width * self.height
    }
}

impl AreaInfo for Circle {
    fn get_area(&self) -> f64 {
        3.14 * self.radius * self.radius
    }
}
impl AreaInfo for Triangle {
    fn get_area(&self) -> f64 {
        0.5 * self.base * self.height
    }
}

fn main() {
    let rec = Rectangle { width: 5.0, height: 8.0 };
    let circle = Circle { radius: 5.0 };
    let triangle = Triangle { base: 2.0, height: 3.0 };

    println!("rectangle area = {}", rec.get_area());
    println!("circle area    = {}", circle.get_area());
    println!("triangle area    = {}", triangle.get_area());
}