fn main() {
    let payload = String::from("Sensitive Server Data");

    let consume_and_send = || {
        let send_data = payload;
        println!("Transmitting: {}", send_data);
        // send data dies here, but also payload
        // as it was reowned.
        // println!("Transmitting: {}", payload);
    };

    consume_and_send();
    
    // consume_and_send(); // err

    // println!("{}", payload); // err
    
}