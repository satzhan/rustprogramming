use std::thread;
use std::time::{Duration, Instant};
use std::sync::mpsc;

fn download_file(file: &str) -> String {
    thread::sleep(Duration::from_millis(100));
    format!("{} downloaded", file)
}

fn resize_image(image: &str) -> String {
    thread::sleep(Duration::from_millis(100));
    format!("{} resized", image)
}

fn main() { // task parallelism
    let files = vec!["file1.txt", "file2.txt", "file3.txt"];
    let images = vec!["image1.jpg", "image2.jpg", "image3.jpg"];

    let start = Instant::now();
    
    let (tx1, rx1) = mpsc::channel();
    let (tx2, rx2) = mpsc::channel();
    
    let download_handle = thread::spawn(move || {
        let result: Vec<String> = files.iter().map(|file| download_file(file)).collect();
        tx1.send(result).unwrap();
    });
    
    let resize_handle = thread::spawn(move || {
        let result: Vec<String> = images.iter().map(|image| resize_image(image)).collect();
        tx2.send(result).unwrap();
    });
    
    // Wait for both tasks to send their data
    let downloaded_files = rx1.recv().unwrap();
    let resized_images = rx2.recv().unwrap();

    download_handle.join().unwrap();
    resize_handle.join().unwrap();
    for x in downloaded_files { println!("{} ", x); } 
    for x in resized_images   { println!("{} ", x); } 

    println!("Time elapsed (native): {:?}", start.elapsed());
}