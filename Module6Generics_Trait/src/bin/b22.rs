

#[derive(Debug)]
struct NewsArticle {
    headline: String,
    author: String,
}

#[derive(Debug)]
struct Tweet {
    username: String,
    content: String,
}

trait Summary {
    fn summarize(&self) -> String;
}

impl Summary for NewsArticle {
    fn summarize(&self) -> String {
        format!("{} by {}", self.headline, self.author)
    }
}

impl Summary for Tweet {
    fn summarize(&self) -> String {
        format!("@{}: {}", self.username, self.content)
    }
}

fn notify(item: impl Summary) {
    println!("Breaking news! {}", item.summarize());
}

fn notify_explicit<T: Summary>(item: T) {
    println!("Update: {}", item.summarize());
}

fn main() {
    let article = NewsArticle {
        headline: String::from("Rust reaches orbit"),
        author: String::from("Gustave"),
    };

    let tweet = Tweet {
        username: String::from("rustacean"),
        content: String::from("Traits are starting to make sense"),
    };

    notify(article);
    notify(tweet);
    notify_explicit(tweet);
}