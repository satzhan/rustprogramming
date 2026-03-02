use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write, Read};

enum FileOp {
    Read,
    Write(String),
    Append(String),
}

fn perform(path: &str, op: FileOp) {
    match op {
        FileOp::Read => {
            let file = File::open(path).expect("Failed to open file");
            let reader = BufReader::new(file);
            for line in reader.lines() {
                println!("{}", line.expect("Failed to read line"));
            }
        }
        FileOp::Write(content) => {
            let mut file = File::create(path).expect("Failed to create file");
            file.write_all(content.as_bytes()).expect("Failed to write to file");
        }
        FileOp::Append(content) => {
            let mut file = OpenOptions::new()
                .append(true)
                .create(true)
                .open(path)
                .expect("Failed to open file");
            file.write_all(content.as_bytes()).expect("Failed to append to file");
        }
    }
}

fn main() {
    let path = "demo.txt";
   perform(path, FileOp::Write("Hello, world!".into()));
   perform(path, FileOp::Append("\nAppended text.".into()));
   perform(path, FileOp::Read);
}