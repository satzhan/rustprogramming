use std::cmp::PartialOrd;

fn max_in_slice<T: PartialOrd + Copy>(items: &[T]) -> Option<T> {
    let mut iter = items.iter().copied();
    let mut best = iter.next()?;

    for item in iter {
        if item > best {
            best = item;
        }
    }

    Some(best)
}

fn main() {
    let nums = [3, 9, 2, 7];
    let floats = [1.2, 8.4, 3.3];
    let letters = ['a', 'z', 'm'];
    let empty: [i32; 0] = [];

    println!("nums    -> {:?}", max_in_slice(&nums));
    println!("floats  -> {:?}", max_in_slice(&floats));
    println!("letters -> {:?}", max_in_slice(&letters));
    println!("empty   -> {:?}", max_in_slice(&empty));
}