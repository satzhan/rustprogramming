fn main() {
    let threshold = 10;

    struct CheckLimitClosure<'a> {
        threshold_ptr: &'a i32,
    }
    impl<'a> CheckLimitClosure<'a> {
        fn call(&self, value: i32) -> bool {
            value > *self.threshold_ptr
        }
    }

    let check_limit_instance = CheckLimitClosure {
        threshold_ptr: &threshold
    };

    println!("Is 15 over limit? {}", check_limit_instance.call(15));
    println!("Is 5 over limit? {}", check_limit_instance.call(5));
}