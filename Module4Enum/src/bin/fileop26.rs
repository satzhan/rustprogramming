use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write, Read};

enum FileOp {
    Read,
    Write(String),
    Append(String),
}

fn perform_file_operation(file_path: &str, operation: FileOp) -> std::io::Result<()> {
    match operation {
        FileOp::Read => {
            let file = File::open(file_path)?;
            let reader = BufReader::new(file);
            for line in reader.lines() {
                println!("{}", line?);
            }
        }
        FileOp::Write(content) => {
            let mut file = File::create(file_path)?;
            file.write_all(content.as_bytes())?;
        }
        FileOp::Append(content) => {
            let mut file = OpenOptions::new()
                .write(true)
                .append(true)
                .open(file_path)?;
            file.write_all(content.as_bytes())?;
        }
    }
    Ok(())
}

fn main() {
    let path = "test_lego.txt";
    perform_file_operation(path, FileOp::Write("Hello, world!".into())).expect("Failed to write to file");
    perform_file_operation(path, FileOp::Append("\nAppended text.".into())).expect("Failed to append to file");
    perform_file_operation(path, FileOp::Read).expect("Failed to read from file");
}