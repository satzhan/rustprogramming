#[derive(PartialEq, Debug)]
enum TrafficLight {
    Red,
    Yellow,
    Green,
}

fn main() {
    let light = TrafficLight::Red;

    if light == TrafficLight::Red {
        println!("Stop");
    } else if light == TrafficLight::Yellow {
        println!("Caution");
    } else if light == TrafficLight::Green {
        println!("Go");
    }

    match light {
        TrafficLight::Red => println!("Stop"),
        TrafficLight::Yellow => println!("Caution"),
        TrafficLight::Green => println!("Go"),
    }
}
