struct Tv {
    channel: String,
}

struct Viewer<'a> {
    tv_reference: &'a Tv,
}

struct RemoteControl<'a> {
    tv_reference: &'a mut Tv,
}

fn main() {
    let mut house_tv = Tv {
        channel: String::from("Sports Network"),
    };

    let alice = Viewer {
        tv_reference: &house_tv,
    };

    // err
    // let bob = RemoteControl {
    //     tv_reference: &mut house_tv,
    // };

    println!("Alice is watching: {}", alice.tv_reference.channel);
    
    // bob.tv_reference.channel = String::from("News");
}