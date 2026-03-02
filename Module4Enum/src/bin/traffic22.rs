enum TrafficLight {
    Red,
    Yellow,
    Green,
}

fn handle_traffic_light(light: TrafficLight) {
    match light {
        TrafficLight::Red => println!("Stop"),
        TrafficLight::Yellow => println!("Caution"),
        TrafficLight::Green => println!("Go"),
    }
}

fn main() {
    let light = TrafficLight::Green;
    handle_traffic_light(light);
}