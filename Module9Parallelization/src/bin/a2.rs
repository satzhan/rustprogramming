use rayon::prelude::*;

fn main() {
    let mut data = vec![1, 2, 3, 4, 5];

    // par_iter_mut() safely splits the data without requiring a Mutex
    data.par_iter_mut().for_each(|x| {
        *x *= 2;
    });
    
    println!("{:?}", data); // [2, 4, 6, 8, 10]
}

