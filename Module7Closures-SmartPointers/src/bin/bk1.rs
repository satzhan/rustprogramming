struct CustomCleanupBox {
    name: String,
}

impl Drop for CustomCleanupBox {
    fn drop(&mut self) {
        println!("🗑️ [DROP TRIGGERED]: Freeing memory for `{}`", self.name);
    }
}

fn main() {
    println!("--- Program Starts ---");

    {
        println!("--> Entering Inner Scope");
        
        let standard_box = Box::new(50);
        println!("Value inside Box: {}", *standard_box);

        let _ptr1 = CustomCleanupBox { name: String::from("Node A") };
        let _ptr2 = CustomCleanupBox { name: String::from("Node B") };
        
        println!("--> Exiting Inner Scope");
    } 

    println!("--- Program Ends ---");
}