enum WeatherCondition {
    Sunny(u32),
    Rainy(u32),
}

fn weather_report(condition: WeatherCondition) {
    match condition {
        WeatherCondition::Sunny(temp) if temp > 30 => println!("It's a hot sunny day! {} degrees.", temp),
        WeatherCondition::Sunny(temp) => println!("It's sunny and {} degrees.", temp),
        WeatherCondition::Rainy(temp) => println!("It's rainy and {} degrees.", temp),
    }
}

fn main() {
    let sunny_day = WeatherCondition::Sunny(32);
    let rainy_day = WeatherCondition::Rainy(22);

    weather_report(sunny_day);
    weather_report(rainy_day);
}
