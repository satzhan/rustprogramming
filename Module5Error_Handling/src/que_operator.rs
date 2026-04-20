use std::fs::File;
use std::io::{self, Read};

fn read_username_from_file() -> Result<String, io::Error> {
    // If open fails, the ? returns the io::Error immediately.
    // If it succeeds, 'mut f' gets the File object.
    let mut f = File::open("username.txt")?;
    
    let mut s = String::new();
    
    // If read_to_string fails, the ? returns the io::Error immediately.
    // If it succeeds, we return the String wrapped in Ok.
    f.read_to_string(&mut s)?;
    
    Ok(s)
}

fn read_username_from_file() -> Result<String, io::Error> {
    let mut s = String::new();
    // Chaining the operations
    File::open("username.txt")?.read_to_string(&mut s)?;
    Ok(s)
}