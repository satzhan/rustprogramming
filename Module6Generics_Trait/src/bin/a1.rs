// Run this to see how a single struct definition can handle multiple types.
#[derive(Debug)]
struct Point<T> {
    x: T,
    y: T,
}

#[derive(Debug)]
struct User<T, U> {
    name: T,
    y: U,
}

fn main() {
    // Both x and y must be the same type (T)
    let mut integer_point = Point { x: 5, y: 10 };
    let mut float_point = Point { x: 1.0, y: 4.0 };
    
    println!("Int Point: {:?}", integer_point);
    println!("Float Point: {:?}", float_point);

    // name and y can be different types (T and U)
    let user1 = User { name: "Vandam", y: 35 };
    let user2 = User { name: "James Bond".to_string(), y: "===> 007" };

    println!("User1: {:?}", user1);
    println!("User2: {:?}", user2);
}