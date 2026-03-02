fn is_valid(s: &str) -> bool {
    let mut st: Vec<char> = Vec::new();
    for c in s.chars() {
        match c {
            '(' | '[' | '{' => st.push(c),
            ')' => if st.pop() != Some('(') { return false; },
            ']' => if st.pop() != Some('[') { return false; },
            '}' => if st.pop() != Some('{') { return false; },
            _ => {}
        }
    }
    st.is_empty()
}

fn main() {
    let test_str = "{[()]}";
    if is_valid(test_str) {
        println!("The string \"{}\" is valid.", test_str);
    } else {
        println!("The string \"{}\" is invalid.", test_str);
    }

    let another_test_str = "{[(])}";
    if is_valid(another_test_str) {
        println!("The string \"{}\" is valid.", another_test_str);
    } else {
        println!("The string \"{}\" is invalid.", another_test_str);
    }
}