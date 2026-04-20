use std::sync::mpsc::{channel, Sender, Receiver};
use std::thread;
use std::time::{Duration, Instant};

enum Message {
    Download(String),
    Process(String),
    Exit,
}

// STAGE 1: Downloader
struct Downloader { tx: Sender<Message> }
impl Downloader {
    fn run(&self, files: &[&str]) {
        for file in files {
            thread::sleep(Duration::from_millis(100)); 
            self.tx.send(Message::Download(format!("{} downloaded", file))).unwrap();
        }
        self.tx.send(Message::Exit).unwrap();
    }
}

// STAGE 2: Processor
struct Processor { // queue logic
    rx: Receiver<Message>, 
    tx: Sender<Message>, // send t the worker pool
}
impl Processor {
    fn run(&self) {
        loop {
            match self.rx.recv().unwrap() {
                Message::Download(file) => {
                    thread::sleep(Duration::from_millis(100)); 
                    self.tx.send(Message::Process(format!("{} processed", file))).unwrap();
                }
                Message::Exit => {
                    self.tx.send(Message::Exit).unwrap();
                    break;
                }
                _ => {}
            }
        }
    }
}

// STAGE 3: Uploader
struct Uploader { rx: Receiver<Message> }
impl Uploader {
    fn run(&self) -> Vec<String> {
        let mut uploaded_files = Vec::new();
        loop {
            match self.rx.recv().unwrap() {
                Message::Process(file) => {
                    thread::sleep(Duration::from_millis(100)); 
                    uploaded_files.push(format!("{} uploaded", file));
                }
                Message::Exit => break,
                _ => {}
            }
        }
        uploaded_files
    }
}

fn main() {
    let files = vec!["file1.txt", "file2.txt", "file3.txt"];
    let start = Instant::now();

    // Wire up the assembly line
    let (downloader_tx, processor_rx) = channel();
    let (processor_tx, uploader_rx) = channel();

    let downloader = Downloader { tx: downloader_tx };
    let processor = Processor { tx: processor_tx, rx: processor_rx };
    let uploader = Uploader { rx: uploader_rx };

    let files_clone = files.clone();
    
    // Start the threads
    let downloader_thread = thread::spawn(move || downloader.run(&files_clone));
    let processor_thread = thread::spawn(move || processor.run());

    // The main thread acts as the final uploader stage
    let uploaded_files_parallel = uploader.run();

    downloader_thread.join().unwrap();
    processor_thread.join().unwrap();

    println!("Pipeline duration: {:?}", start.elapsed());
}