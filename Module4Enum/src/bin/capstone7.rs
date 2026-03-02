use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::Path;    

enum Op {
    List(String),
    Display(String),
    Create(String, String),
    Remove(String),
    Pwd,
    Exit,
}

fn read_line(prompt: &str) -> String {
    print!("{}", prompt);
    io::stdout().flush().unwrap();

    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    input.trim().to_string()
}

fn perform(op: Op) {
    match op {
        Op::List(path) => {
            let entries = fs::read_dir(path).expect("Failed to read directory");
            for entry in entries {
                let entry = entry.expect("Failed to read entry");
                println!("{}", entry.file_name().to_string_lossy());
            }
        }
        Op::Display(path) => {
            let content = fs::read_to_string(path).expect("Failed to read file");
            println!("{}", content);
        }
        Op::Create(path, content) => {
            fs::write(path, content).expect("Failed to create file");
            println!("File created successfully.");
        }
        Op::Remove(path) => {
            fs::remove_file(path).expect("Failed to remove file");
            println!("File removed successfully.");
        }
        Op::Pwd => {
            let current_dir = env::current_dir().expect("Failed to get current directory");
            println!("Current directory: {}", current_dir.display());
        }
        Op::Exit => {
            println!("Exiting...");
        }
    }
}

fn main() {
    loop {
        println!("Choose an operation:");
        println!("1. List directory");
        println!("2. Display file content");
        println!("3. Create a file");
        println!("4. Remove a file");
        println!("5. Print working directory");
        println!("6. Exit");

        let choice = read_line("Enter your choice (1-6): ");

        match choice.as_str() {
            "1" => {
                let path = read_line("Enter the directory path: ");
                perform(Op::List(path));
            }
            "2" => {
                let path = read_line("Enter the file path: ");
                perform(Op::Display(path));
            }
            "3" => {
                let path = read_line("Enter the file path to create: ");
                let content = read_line("Enter the content for the file: ");
                perform(Op::Create(path, content));
            }
            "4" => {
                let path = read_line("Enter the file path to remove: ");
                perform(Op::Remove(path));
            }
            "5" => {
                perform(Op::Pwd);
            }
            "6" => {
                perform(Op::Exit);
                break;
            }
            _ => {
                println!("Invalid choice.");
            }
        }
    }
    println!("Goodbye!");
}