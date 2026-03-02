enum Weather {
    Sunny(u32),
    Rainy(u32),
    Cloudy(u32),
    Snowy(u32),
}

fn report(w: Weather) {
    match w {
        Weather::Sunny(temp) if temp > 30 => println!("It's a hot sunny day!"),
        Weather::Sunny(temp) => println!("It's sunny {}°C", temp),
        Weather::Rainy(temp) => println!("It's rainy with a temperature of {}°C", temp),
        Weather::Cloudy(temp) => println!("It's cloudy with a temperature of {}°C", temp),
        Weather::Snowy(temp) => println!("It's snowy with a temperature of {}°C", temp),
    }
}

fn main() {
    report(Weather::Cloudy(22));
    report(Weather::Sunny(35));
}