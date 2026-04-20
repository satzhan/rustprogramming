use std::thread;

fn main() {
    let data = vec![1, 2, 3];

    thread::spawn(|| {
        println!("Thread reading: {:?}", data); //err
    });

    // thread::spawn(move || {
    //     println!("Thread reading: {:?}", data);
    // }).join().unwrap();
}