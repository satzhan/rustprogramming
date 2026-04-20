#[derive(Debug)]
struct Point<T> {
    x: T,
    y: T,
}

#[derive(Debug)]
struct Pair<A, B> {
    first: A,
    second: B,
}

fn main() {
    let p1 = Point { x: 3, y: 7 };
    let p2 = Point { x: 1.5, y: 9.2 };
    let p3 = Point { x: 'a', y: 'z' };

    println!("p1 = {:?}", p1);
    println!("p2 = {:?}", p2);
    println!("p3 = {:?}", p3);

    let a = Pair { first: 10, second: 20 };
    let b = Pair { first: "age", second: 33 };
    let c = Pair { first: 'x', second: 2.5 };

    println!("a = {:?}", a);
    println!("b = {:?}", b);
    println!("c = {:?}", c);
}