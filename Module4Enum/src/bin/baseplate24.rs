#[derive(Debug)]
enum Genre {
    Fiction,
    NonFiction,
}

struct Book {
    title: String,
    author: String,
    genre: Genre,
}

fn main() {
    let book = Book {
        title: String::from("1984"),
        author: String::from("George Orwell"),
        genre: Genre::Fiction,
    };

    println!("Book title: {}", book.title);
    println!("Book author: {}", book.author);
    println!("Book genre: {:?}", book.genre);
}