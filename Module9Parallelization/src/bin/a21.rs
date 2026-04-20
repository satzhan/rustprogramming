use rayon::prelude::*;
use std::time::Instant;

fn main() {
    let size = 100_000_000;
    println!("Generating {} elements...", size);
    
    // 1. Create two identical datasets of f64 numbers
    let mut data_seq: Vec<f64> = (0..size).map(|x| x as f64).collect();
    let mut data_par = data_seq.clone(); // Clone to ensure a fair test

    println!("Starting sequential execution...");
    let start_seq = Instant::now();
    
    // 2. Sequential processing
    data_seq.iter_mut().for_each(|x| {
        // We use complex math to simulate a heavy workload
        *x = x.sqrt() * x.sin();
    });
    let duration_seq = start_seq.elapsed();
    println!("Sequential time: {:?}", duration_seq);

    println!("Starting parallel execution...");
    let start_par = Instant::now();
    
    // 3. Parallel processing with Rayon
    data_par.par_iter_mut().for_each(|x| {
        *x = x.sqrt() * x.sin();
    });
    let duration_par = start_par.elapsed();
    println!("Parallel time: {:?}", duration_par);

    // 4. Calculate the speedup multiplier
    let speedup = duration_seq.as_secs_f64() / duration_par.as_secs_f64();
    println!("Rayon was {:.2}x faster!", speedup);
}