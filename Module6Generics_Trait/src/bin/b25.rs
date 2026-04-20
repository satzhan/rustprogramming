trait Speak {
    fn speak(&self) -> String;
}

#[derive(Debug)]
struct Dog {
    name: String,
}

#[derive(Debug)]
struct Cat {
    name: String,
}

impl Speak for Dog {
    fn speak(&self) -> String {
        format!("Dog {} says woof", self.name)
    }
}

impl Speak for Cat {
    fn speak(&self) -> String {
        format!("Cat {} says meow", self.name)
    }
}

// Can accept two possibly different types,
// as long as both implement Speak.
fn talk_any(a: &impl Speak, b: &impl Speak) {
    println!("first  = {}", a.speak());
    println!("second = {}", b.speak());
}

// Requires both arguments to be the SAME concrete type T.
fn talk_same<T: Speak>(a: &T, b: &T) {
    println!("same-type pair:");
    println!("first  = {}", a.speak());
    println!("second = {}", b.speak());
}

fn main() {
    let dog1 = Dog {
        name: String::from("Rex"),
    };
    let dog2 = Dog {
        name: String::from("Bolt"),
    };
    let cat1 = Cat {
        name: String::from("Mimi"),
    };
    let cat2 = Cat {
        name: String::from("Timi"),
    };

    println!("--- talk_any with Dog + Cat ---");
    talk_any(&dog1, &cat1);

    println!("\n--- talk_same with Dog + Dog ---");
    talk_same(&dog1, &dog2);
    talk_same(&cat1, &cat2);

    // This would NOT compile:
    // talk_same(&dog1, &cat1);
}