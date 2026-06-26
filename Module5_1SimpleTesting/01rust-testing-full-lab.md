# Comprehensive Rust Testing Lab: Using main.rs

## Introduction
This lab will guide you through the process of writing and running tests in Rust, focusing exclusively on using `main.rs`. By the end, you'll have a solid understanding of basic testing concepts in Rust.

## Setup
1. Open your terminal.
2. Create a new Rust project:
   ```
   cargo new rust_testing_lab
   cd rust_testing_lab
   ```
3. Open `src/main.rs` in your preferred text editor.

## Lab 1: Your First Test

1. Replace the contents of `src/main.rs` with:

```rust
fn add(a: i32, b: i32) -> i32 {
    a + b
}

fn main() {
    println!("2 + 2 = {}", add(2, 2));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add() {
        assert_eq!(add(2, 2), 4);
    }
}
```

2. Run your tests:
   ```
   cargo test
   ```
3. You should see output indicating that the test passed.

## Lab 2: Testing Edge Cases

1. Add these tests to the `tests` module:

```rust
#[test]
fn test_add_negative() {
    assert_eq!(add(-2, -2), -4);
}

#[test]
fn test_add_zero() {
    assert_eq!(add(0, 0), 0);
}
```

2. Run the tests. They should all pass.

## Lab 3: Introducing assert! and assert_ne!

1. Add a new function to `main.rs`:

```rust
fn is_even(n: i32) -> bool {
    n % 2 == 0
}
```

2. Add these tests:

```rust
#[test]
fn test_is_even() {
    assert!(is_even(2));
    assert!(!is_even(3));
}

#[test]
fn test_not_equal() {
    assert_ne!(add(2, 2), 5);
}
```

3. Run the tests. They should all pass.

## Lab 4: Testing for Panics

1. Add a new function:

```rust
fn divide(a: i32, b: i32) -> i32 {
    if b == 0 {
        panic!("Cannot divide by zero");
    }
    a / b
}
```

2. Add this test:

```rust
#[test]
#[should_panic(expected = "Cannot divide by zero")]
fn test_divide_by_zero() {
    divide(10, 0);
}
```

3. Run the tests. They should all pass.

## Lab 5: Parameterized Tests

1. Add this test to explore multiple cases:

```rust
#[test]
fn test_add_multiple() {
    let test_cases = vec![
        (1, 1, 2),
        (0, 0, 0),
        (-1, 1, 0),
        (100, -50, 50)
    ];
    
    for (a, b, expected) in test_cases {
        assert_eq!(add(a, b), expected, "Failed on input ({}, {})", a, b);
    }
}
```

2. Run the tests. They should all pass.

## Lab 6: Custom Failure Messages

1. Add this test to demonstrate custom failure messages:

```rust
#[test]
fn test_is_even_verbose() {
    let number = 5;
    assert!(
        is_even(number),
        "Expected {} to be even, but it was odd",
        number
    );
}
```

2. Run the tests. This new test should fail with your custom message.

## Lab 7: Organizing Tests

1. Add this nested module to your `tests` module:

```rust
mod arithmetic_tests {
    use super::*;

    #[test]
    fn test_complex_add() {
        assert_eq!(add(add(1, 2), add(3, 4)), 10);
    }

    #[test]
    fn test_even_arithmetic() {
        assert!(is_even(add(2, 2)));
        assert!(!is_even(add(2, 3)));
    }
}
```

2. Run the tests. All tests, including those in the nested module, should run.

## Final main.rs

Your final `main.rs` should look like this:

```rust
fn add(a: i32, b: i32) -> i32 {
    a + b
}

fn is_even(n: i32) -> bool {
    n % 2 == 0
}

fn divide(a: i32, b: i32) -> i32 {
    if b == 0 {
        panic!("Cannot divide by zero");
    }
    a / b
}

fn main() {
    println!("2 + 2 = {}", add(2, 2));
    println!("Is 4 even? {}", is_even(4));
    println!("10 / 2 = {}", divide(10, 2));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add() {
        assert_eq!(add(2, 2), 4);
    }

    #[test]
    fn test_add_negative() {
        assert_eq!(add(-2, -2), -4);
    }

    #[test]
    fn test_add_zero() {
        assert_eq!(add(0, 0), 0);
    }

    #[test]
    fn test_is_even() {
        assert!(is_even(2));
        assert!(!is_even(3));
    }

    #[test]
    fn test_not_equal() {
        assert_ne!(add(2, 2), 5);
    }

    #[test]
    #[should_panic(expected = "Cannot divide by zero")]
    fn test_divide_by_zero() {
        divide(10, 0);
    }

    #[test]
    fn test_add_multiple() {
        let test_cases = vec![
            (1, 1, 2),
            (0, 0, 0),
            (-1, 1, 0),
            (100, -50, 50)
        ];
        
        for (a, b, expected) in test_cases {
            assert_eq!(add(a, b), expected, "Failed on input ({}, {})", a, b);
        }
    }

    #[test]
    fn test_is_even_verbose() {
        let number = 5;
        assert!(
            is_even(number),
            "Expected {} to be even, but it was odd",
            number
        );
    }

    mod arithmetic_tests {
        use super::*;

        #[test]
        fn test_complex_add() {
            assert_eq!(add(add(1, 2), add(3, 4)), 10);
        }

        #[test]
        fn test_even_arithmetic() {
            assert!(is_even(add(2, 2)));
            assert!(!is_even(add(2, 3)));
        }
    }
}
```

## Conclusion

You've now explored various aspects of testing in Rust:
- Basic assertions with `assert_eq!`, `assert!`, and `assert_ne!`
- Testing for panics with `#[should_panic]`
- Parameterized tests
- Custom failure messages
- Organizing tests with nested modules

All of this was done within a single `main.rs` file, demonstrating how you can start testing in Rust without needing to set up complex project structures.


```
use std::error::Error;
use std::fs;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Direction {
    Left,
    Right,
}

#[derive(Debug, Clone)]
struct Move {
    direction: Direction,
    amount: i32,
}


fn parse(input: &str) -> Result <Vec<Move>, Box<dyn Error>> {
    let mut moves = Vec::new();
    for line in input.lines() {
        if line.is_empty() { continue; }
        let first = line.chars().next().unwrap();
        let direction = match first {
            'R' => Direction::Right,
            'L' => Direction::Left,
            other => return Err(format!("Unknown {:?}", other).into()),
        };
        let amount: i32 = line[1..].parse()?;
        moves.push(Move{ direction, amount });
    }
    Ok(moves)
}

fn solve(moves: &[Move]) -> i32 {
    let mut dial = 50;
    let mut count = 0;
    for m in moves {
        dial = match m.direction{
            Direction::Right => { 
                // what ever you want
                (dial + m.amount) % 100
            }
            Direction::Left  => {
                // else what you want
                (dial - m.amount) % 100
            }
        };
        if dial == 0 { count += 1; }
    }
    count
}
fn solve_part2_bf(moves: &[Move]) -> i32 {
    let mut dial = 50;
    let mut count = 0;
    for m in moves {
        if m.direction == Direction::Right {
            for _ in 0..m.amount {
                dial = (dial + 1) % 100;
                if dial == 0 { count += 1; }
            }
        } else {
             for _ in 0..m.amount {
                dial = (dial - 1) % 100;
                if dial == 0 { count += 1; }
            }
        }
    }
    return count;
}

fn main() -> Result<(), Box<dyn Error>> {
    let input = fs::read_to_string("test.txt")?;
    let moves = parse(&input)?;
    let count = solve(&moves);
    let count2 = solve_part2_bf(&moves);
    println!("Result: {count} :: {count2}");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::RngExt;
    use std::time::Instant;

    #[test]
    fn test1() {
        assert_eq!(2, 2);
        let mut moves = Vec::new();
        moves.push(Move{direction: Direction::Right, amount: 50});
        let sol = solve_part2_bf(&moves);
        assert_eq!(sol, 1);
    }

    #[test]
    fn test2() {
        let input = "L50\nR100";
        let input = fs::read_to_string("test.txt").unwrap();
        let moves = parse(&input).unwrap();
        let count = solve_part2_bf(&moves);
        assert_eq!(count, 6);
    }

    fn generate_moves() ->Vec<Move> {
        let mut moves = Vec::new();
        let mut rng = rand::rng();
        let n = rng.random_range(0..100); // number of tests
        for _ in 0..n {
            let direction = if rng.random_range(0..2) == 0 {
                Direction::Right
            } else {
                Direction::Left
            };
            let amount = rng.random_range(0..1000);
            moves.push(Move{ direction, amount });
        }
        moves
    }
    #[test]
    fn test3() {
        let mut rng = rand::rng();
        let number = rng.random_range(0..100);
        let start = Instant::now();
        let input = fs::read_to_string("test.txt").unwrap();
        let moves = parse(&input).unwrap();
        let count = solve_part2_bf(&moves);
        let time = start.elapsed();
        println!("Time spent {}", time.as_secs_f64());
        assert_eq!(count, 6);
        assert!(number < 100);
    }

    // #[test]
    // fn test4() {
    //     let moves = generate_moves();
    //     let count_bf = solve_part2_bf(&moves);
    //     let count_non_bf = solve_part2_non_bf(&moves);
    //     assert_eq!(count_bf, count_non_bf);
        
    // }
}
```


```
rustc --test main.rs
./main  # (or .\main.exe on Windows)

rustc --test src/bin/my_script.rs
./my_script  # (or .\my_script.exe on Windows)
```

if you're running cargo init
```
cargo test

cargo test --bin my_script
```

To run a paricular test and ask not to remove println! outputs
```
cargo test test4 -- --nocapture
```
