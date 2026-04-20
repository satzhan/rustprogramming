fn main() {
    let mut violations = 0;
    let threshold = 10;

    struct CheckAndCountClosure<'a, 'b> {
        threshold_ptr: &'a i32,
        violations_ptr: &'b mut i32,
    }
    impl<'a, 'b> CheckAndCountClosure<'a, 'b> {
        fn call_mut(&mut self, value: i32) -> bool {
            if value > *self.threshold_ptr {
                *self.violations_ptr += 1;
                true
            } else {
                false
            }
        }
    }

    let mut manual_closure = CheckAndCountClosure {
        threshold_ptr: &threshold,
        violations_ptr: &mut violations,
    };

    println!(" is over 15? {}", manual_closure.call_mut(15));
    println!(" is over 12? {}", manual_closure.call_mut(12));

    drop(manual_closure);
    println!("Total violations: {}", violations);
}