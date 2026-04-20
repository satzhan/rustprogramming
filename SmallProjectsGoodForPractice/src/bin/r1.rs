struct ResourcePool {
    red: i32,
    green: i32,
}

impl ResourcePool {
    fn apply_strategy(&mut self) -> usize {
        let mut pulls = 0;

        while self.red > 0 || self.green > 0 {
            pulls += 1;
            
            if self.red >= 8 && (self.red - 8) >= (self.green * 2) {
                self.red -= 8;
                println!("Pull {}: 8 Red. Remaining: R:{}, G:{}", pulls, self.red, self.green);
            } else if self.red >= 5 && self.green >= 1 {
                self.red -= 5;
                self.green -= 1;
                println!("Pull {}: 5 Red, 1 Green. Remaining: R:{}, G:{}", pulls, self.red, self.green);
            } else if self.red >= 2 && self.green >= 2 {
                self.red -= 2;
                self.green -= 2;
                println!("Pull {}: 2 Red, 2 Green. Remaining: R:{}, G:{}", pulls, self.red, self.green);
            } else {
                println!("Deadlock or remaining resources cannot be pulled perfectly.");
                break;
            }
        }
        pulls
    }
}

fn main() {
    let mut pool = ResourcePool { red: 70, green: 30 };
    let total = pool.apply_strategy();
    println!("Total pulls: {}", total);
}