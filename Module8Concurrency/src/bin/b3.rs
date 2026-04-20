use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::Duration;

// Define a type for our processing function
type ProcessingFn = fn(usize, i32) -> i32;

// Define a function that processes data by squaring it
fn square_data(id: usize, value: i32) -> i32 {
    println!("Worker {} squaring value: {}", id, value);
    thread::sleep(Duration::from_millis(100));
    
    let result = value * value;
    
    println!("Worker {} finished squaring. Result: {}", id, result);
    result
}

// Define another processing function that doubles data
fn double_data(id: usize, value: i32) -> i32 {
    println!("Worker {} doubling value: {}", id, value);
    thread::sleep(Duration::from_millis(50));
    
    let result = value * 2;
    
    println!("Worker {} finished doubling. Result: {}", id, result);
    result
}

// This function creates worker threads and takes a processing function as a parameter
fn create_worker_pool(num_workers: usize, processor: ProcessingFn) -> (Vec<thread::JoinHandle<()>>, mpsc::Sender<i32>, mpsc::Receiver<i32>) {
    // Create channels for communication
    let (task_tx, task_rx) = mpsc::channel();  // For sending tasks to workers
    let (result_tx, result_rx) = mpsc::channel();  // For receiving results
    
    // Wrap the task receiver in Arc<Mutex> to share among workers
    let task_rx = Arc::new(Mutex::new(task_rx));
    
    // Create worker threads
    let mut handles = vec![];
    
    for worker_id in 1..=num_workers {
        let task_rx_clone = Arc::clone(&task_rx);
        let result_tx_clone = result_tx.clone();
        
        // Create a thread that uses the passed processing function
        let handle = thread::spawn(move || {
            loop {
                // Get a task from the channel
                let value = {
                    let receiver = task_rx_clone.lock().unwrap();
                    match receiver.recv() {
                        Ok(val) => val,
                        Err(_) => break, // Channel closed
                    }
                };
                
                if value == -1 {
                    println!("Worker {} received termination signal", worker_id);
                    break;
                }
                
                // Call the processing function passed as a parameter
                let result = processor(worker_id, value);
                
                // Send result back to main thread
                result_tx_clone.send(result).unwrap();
            }
            
            println!("Worker {} shutting down", worker_id);
        });
        
        handles.push(handle);
    }
    
    // Return the handles, task sender, and result receiver so the caller can use them
    (handles, task_tx, result_rx)
}

fn main() {
    println!("=== Starting worker pool with squaring function ===");
    
    // Create the worker pool using the square_data function
    // Now we get the task sender back from the function
    const N:usize = 5; // workers
    const M:i32 = 10; // tasks
    let (handles, task_tx, results) = create_worker_pool(N, square_data);
    
    // Generate some test data
    for i in 1..=M {
        task_tx.send(i).unwrap(); // instant
        // sleep(20) ms
        println!("Main: Sent value {} for processing", i);
    }
    
    // Send termination signal to all workers
    for _ in 0..N {
        task_tx.send(-1).unwrap();
    }
    
    // Drop the sender to close the channel
    drop(task_tx);
    
    // Collect and display results
    let mut result_count = 0;
    while result_count < M {
        match results.recv() {
            Ok(result) => {
                println!("Main: Received result: {}", result);
                result_count += 1;
            }
            Err(_) => break,
        }
    }
    
    // Wait for all workers to finish
    for handle in handles {
        handle.join().unwrap();
    }
    
    println!("\n=== Starting worker pool with doubling function ===");
    
    // Create another worker pool using the double_data function
    let (handles, task_tx, results) = create_worker_pool(2, double_data);
    
    // Generate some test data
    for i in 1..=10 {
        task_tx.send(i).unwrap();
        println!("Main: Sent value {} for processing", i);
    }
    
    // Send termination signal to all workers
    for _ in 0..2 {
        task_tx.send(-1).unwrap();
    }
    
    // Drop the sender to close the channel
    drop(task_tx);
    
    // Collect and display results
    let mut result_count = 0;
    while result_count < 10 {
        match results.recv() {
            Ok(result) => {
                println!("Main: Received result: {}", result);
                result_count += 1;
            }
            Err(_) => break,
        }
    }
    
    // Wait for all workers to finish
    for handle in handles {
        handle.join().unwrap();
    }
    
    println!("All workers have completed their tasks");
}