trait AreaInfo {
    fn get_area(&self) -> f64;
}

#[derive(Debug)]
struct Rectangle {
    width: f64,
    height: f64,
}

#[derive(Debug)]
struct Circle {
    radius: f64,
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

fn main() {
    let rec = Rectangle {
        width: 5.0,
        height: 8.0,
    };

    let circle = Circle {
        radius: 3.0,
    };

    let shapes: Vec<&dyn AreaInfo> = vec![&rec, &circle];

    for shape in shapes {
        println!("area = {}", shape.get_area());
    }
}