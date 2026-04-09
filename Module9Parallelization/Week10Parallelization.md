# A Practical Guide to Parallelism in Rust

## The Three Pillars of Parallelism

Before diving into code, we need to distinguish how we divide our workloads.

* **Data Parallelism:** Dividing the *data* among multiple threads and performing the same operation on each chunk concurrently. 
* **Task Parallelism:** Dividing the *work* into independent, distinct tasks that can be executed concurrently.
* **Pipeline Parallelism:** Dividing the work into a series of dependent stages, where the output of one stage becomes the input for the next. 

> **Important Concepts:** > * **Computational Graphs:** A way to map out dependencies and reason about the time complexity of parallel programs (conceptually similar to [LeetCode 113: Path Sum II](https://leetcode.com/problems/path-sum-ii/)).
> * **The Actor Model:** A design pattern where isolated "actors" communicate purely through message passing. This is highly suited for **Pipeline Parallelism**.

---

## 1. Data Parallelism: The Locking Problem vs. Rayon

The most common mistake in native data parallelism is creating bottlenecks through over-locking. 

### The Native Approach (And Its Flaw)

```rust
use std::sync::{Arc, Mutex};
use std::thread;

fn data_parallelism_native() {
    let data = Arc::new(Mutex::new(vec![1, 2, 3, 4, 5]));
    let len = data.lock().unwrap().len();
    let mid = len / 2;

    let left_data = Arc::clone(&data);
    let left_handle = thread::spawn(move || {
        let mut left = left_data.lock().unwrap();
        for x in &mut left[..mid] {
            *x *= 2;
        }
    });

    let right_data = Arc::clone(&data);
    let right_handle = thread::spawn(move || {
        let mut right = right_data.lock().unwrap();
        for x in &mut right[mid..] {
            *x *= 2;
        }
    });

    left_handle.join().unwrap();
    right_handle.join().unwrap();

    println!("{:?}", *data.lock().unwrap()); // [2, 4, 6, 8, 10]
}
```

> **The Problem:** Even though we conceptually split the data into a left and right half, both threads must acquire a lock on the *entire* vector to update their values. Only one thread can actually work at a time, creating unnecessary overhead. 

### The Rayon Solution

Rayon creates a separate scope to eliminate the need for locking entirely, and uses **work stealing** to dynamically balance the workload across threads.

```rust
use rayon::prelude::*;

fn data_parallelism_rayon() {
    let mut data = vec![1, 2, 3, 4, 5];

    // par_iter_mut() safely splits the data without requiring a Mutex
    data.par_iter_mut().for_each(|x| {
        *x *= 2;
    });
    
    println!("{:?}", data); // [2, 4, 6, 8, 10]
}

fn rayon_iterators_real_power() {
    let wiki_txt = "Parallel computing is a type of computation in which many calculations or processes are carried out simultaneously...";
    
    let words: Vec<_> = wiki_txt.split_whitespace().collect();
    
    // A parallel iterator does everything a regular iterator does, but concurrently.
    let words_with_p: Vec<_> = words
        .par_iter()
        .filter(|val| val.find('p').is_some()) 
        .collect();
        
    println!("All words with letter p: {:?}", words_with_p);
}
```

---

## 2. Task Parallelism: Independent Workloads

When you have completely different tasks (like downloading a file vs. resizing an image), you can run them at the same time and join the results later.

### Native Task Parallelism (Using `mpsc`)

```rust
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

fn task_parallelism_native() {
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

    println!("Time elapsed (native): {:?}", start.elapsed());
}
```

### Rayon Task Parallelism (Using `join`)

Rayon simplifies this dramatically with `rayon::join`, which takes two closures and executes them concurrently.

```rust
use rayon::prelude::*;

fn task_parallelism_rayon() {
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
```

---

## 3. Pipeline Parallelism: The Actor Model approach

Pipeline parallelism is the most complex. It requires building an assembly line where workers (threads) pass data to the next stage as soon as they finish their specific task. We achieve this using message-passing channels (`mpsc`).

```rust
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
struct Processor {
    tx: Sender<Message>,
    rx: Receiver<Message>,
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

fn pipeline_parallelism() {
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
```

> **The Next Step:** While threads and channels work for this assembly line, to build *true* pipeline parallelism at scale, you eventually need to transition into asynchronous programming.
