fn main() {
    let payload = String::from("sensitive server data");

    struct ConsumeClosure {
        payload: String,
    }

    impl ConsumeClosure {
        fn call_once(self) {
            println!("Transmitting: {}", self.payload);
        }
    }
    let manual_closure = ConsumeClosure {
        payload: payload,
    };
    // println!("{}", payload);
    manual_closure.call_once();
    // manual_closure.call_once();
}