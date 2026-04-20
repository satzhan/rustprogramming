use std::cmp::PartialOrd;

fn max_ref_in_slice<T: PartialOrd>(items: &[T]) -> Option<&T> {
    let mut iter = items.iter();
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
    let letters = ['a', 'z', 'm'];
    let words = [
        String::from("pear"),
        String::from("watermelon"),
        String::from("apple"),
    ];
    let empty: [String; 0] = [];

    println!("nums    -> {:?}", max_ref_in_slice(&nums));
    println!("letters -> {:?}", max_ref_in_slice(&letters));
    println!("words   -> {:?}", max_ref_in_slice(&words));
    println!("empty   -> {:?}", max_ref_in_slice(&empty));
}