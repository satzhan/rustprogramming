struct Tv {
    channel: String,
}

fn main() {
    let mut house_tv = Tv {
        channel: String::from("Sports Network"),
    };

    let alice_initial_view = &house_tv;
    println!("Alice initially sees: {}", alice_initial_view.channel);

    let bob_remote = &mut house_tv;
    bob_remote.channel = String::from("Movie Channel");

    let alice_view = &house_tv;
    let bob_view = &house_tv;
    let guest_view = &house_tv;

    println!("Alice sees: {}", alice_view.channel);
    println!("Bob sees:   {}", bob_view.channel);
    println!("Guest sees: {}", guest_view.channel);
}