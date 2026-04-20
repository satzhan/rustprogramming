trait Speak {
    fn speak(&self) -> String;
}

struct Dog {
    name: String,
}

struct Cat {
    name: String,
}
struct Robot {
    id: i32,
}
impl Speak for Dog {
    fn speak(&self) -> String {
        format!("Dog {} says woof", self.name)
    }
}
impl Speak for Robot {
    fn speak(&self) -> String {
        format!("Robot {} says beep", self.id)
    }
}
impl Speak for Cat {
    fn speak(&self) -> String {
        format!("Cat {} says meow", self.name)
    }
}

// 1) impl Trait
fn announce(item: impl Speak) {
    println!("announce: {}", item.speak());
}

// 2) explicit generic bound
fn announce_generic<T: Speak>(item: T) {
    println!("announce_generic: {}", item.speak());
}

// 3) trait object
fn announce_all(items: Vec<&dyn Speak>) {
    for item in items {
        println!("announce_all: {}", item.speak());
    }
}

fn main() {
    let dog = Dog {
        name: String::from("Rex"),
    };

    let cat = Cat {
        name: String::from("Mimi"),
    };
    let robot = Robot {
        id: 42,
    };

    announce(dog);

    let cat2 = Cat {
        name: String::from("Luna"),
    };
    announce_generic(cat2);

    let dog2 = Dog {
        name: String::from("Bolt"),
    };
    let cat3 = Cat {
        name: String::from("Nina"),
    };

    let animals: Vec<&dyn Speak> = vec![&dog2, &cat3, &robot];
    announce_all(animals);
}