use std::thread;

fn main() { // just split data
    let mut data = vec![1, 2, 3, 4, 5, 6];

    thread::scope(|s| {
        let (left, right) = data.split_at_mut(3);

        s.spawn(move || {
            for x in left {
                *x *= 2;
            }
        });

        s.spawn(move || {
            for x in right {
                *x *= 2;
            }
        });
        
    });

    println!("{data:?}");
}