use std::fs;
use std::io::{self, Write};
use std::path::Path;

enum FileOperation {
    Create(String),
    Rename(String, String),
}

fn read_line(prompt: &str) -> String {
    print!("{}", prompt);
    io::stdout().flush().unwrap();

    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    input.trim().to_string()
}

fn perform(op: FileOperation) {
    match op {
        FileOperation::Create(filename) => {
            if Path::new(&filename).exists() {
                println!("File '{}' already exists.", filename);
            } else {
                fs::File::create(&filename).expect("Failed to create file");
                println!("File '{}' created successfully.", filename);
            }
        }
        FileOperation::Rename(old_name, new_name) => {
            if !Path::new(&old_name).exists() {
                println!("File '{}' does not exist.", old_name);
            } else if Path::new(&new_name).exists() {
                println!("File '{}' already exists.", new_name);
            } else {
                fs::rename(&old_name, &new_name).expect("Failed to rename file");
                println!("File '{}' renamed to '{}'.", old_name, new_name);
            }
        }
    }
}

fn main() {
    println!("Choose an operation:");
   println!("1. Create a file");
   println!("2. Rename a file");

   let choice = read_line("Enter your choice (1 or 2): ");

   match choice.as_str() {
       "1" => {
           let filename = read_line("Enter the filename to create: ");
           perform(FileOperation::Create(filename));
       }
       "2" => {
           let old_name = read_line("Enter the current filename: ");
           let new_name = read_line("Enter the new filename: ");
           perform(FileOperation::Rename(old_name, new_name));
       }
       _ => {
           println!("Invalid choice.");
       }
   }
}