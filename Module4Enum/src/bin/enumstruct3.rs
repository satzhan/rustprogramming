#[derive(Debug)]
enum Genre {
    Fiction,
    NonFiction,
    ScienceFiction,
    Fantasy,
}

#[derive(Debug)]
struct Book {
    title: String,
    author: String,
    genre: Genre,
}

fn main() {
    let b = Book {
        title: String::from("The Hobbit"),
        author: String::from("J.R.R. Tolkien"),
        genre: Genre::Fantasy,
    };
    println!("{:?}", b.genre);
}