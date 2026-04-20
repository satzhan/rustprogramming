#[warn(unused)]

use std::fs::File;
use std::io::{self, Read}; // Read trait is required for read_to_string

fn read_username_from_file() -> Result<String, io::Error> {
    let f = File::open("username.txt");
    
    // Manual routing: extract the file, or early-return the error
    let mut f = match f {
        Ok(file) => file,
        Err(e) => return Err(e), 
    };
    
    let mut s = String::new();
    
    // Manual routing: extract the string, or return the error
    match f.read_to_string(&mut s) {
        Ok(_) => Ok(s),
        Err(e) => Err(e),
    }
}

fn main() {
    let f = File::open("hello.txt");

    let _f = match f {
        Ok(file) => file,
        Err(error) => {
            panic!("Problem opening the file: {:?}", error)
        },
    };
}