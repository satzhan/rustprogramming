use rayon::prelude::*;
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
fn main() {
    let files = vec!["file1.txt", "file2.txt", "file3.txt"];
    let images = vec!["image1.jpg", "image2.jpg", "image3.jpg"];

    let start = Instant::now();
    
    // Join blocks until both closures complete
    let (downloaded_files, resized_images) = rayon::join(
        || files.par_iter().map(|file| download_file(file)).collect::<Vec<_>>(),
        || images.par_iter().map(|image| resize_image(image)).collect::<Vec<_>>(),
    );
    
    println!("Time elapsed (rayon): {:?}", start.elapsed());
}