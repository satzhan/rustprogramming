use chrono::{DateTime, Utc, Duration};

enum BookStatus {
    Available,
    CheckedOut(DateTime<Utc>), // Due date
    InRepair(String), // Notes on the repair
}

struct Book {
    title: String,
    author: String,
    status: BookStatus,
}

fn display_book_status(book: &Book) {
    match &book.status {
        BookStatus::Available => println!("{} by {} is available for borrowing.", book.title, book.author),
        BookStatus::CheckedOut(due_date) => {
            println!("{} by {} is checked out. Due date: {}", 
                     book.title, book.author, due_date.format("%Y-%m-%d %H:%M:%S"));
        },
        BookStatus::InRepair(notes) => println!("{} by {} is in repair. Notes: {}", book.title, book.author, notes),
    }
}

fn main() {
    let due_date = Utc::now() + Duration::days(14); // 14 days from now
    let book1 = Book {
        title: String::from("The Rust Programming Language"),
        author: String::from("Steve Klabnik and Carol Nichols"),
        status: BookStatus::CheckedOut(due_date),
    };

    let book2 = Book {
        title: String::from("Clean Code"),
        author: String::from("Robert C. Martin"),
        status: BookStatus::Available,
    };

    let book3 = Book {
        title: String::from("Design Patterns"),
        author: String::from("Erich Gamma et al."),
        status: BookStatus::InRepair(String::from("Broken spine, needs rebinding")),
    };

    display_book_status(&book1);
    display_book_status(&book2);
    display_book_status(&book3);
}