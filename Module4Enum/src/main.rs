enum Day {Sun, Mon, Tue, Wed, Thu, Fri, Sat}

fn schedule(d: Day) -> &'static str {
    match d {
        Day::Sun => "Go to the beach",
        Day::Mon => "Go to work",
        Day::Tue => "Go to work",
        Day::Wed => "Go to work",
        Day::Thu => "Go to work",
        Day::Fri => "Go to work",
        Day::Sat => "Go to the park",
    }
}

fn main() {
    let today = Day::Fri;
    println!("Today's schedule: {}", schedule(today));
}
