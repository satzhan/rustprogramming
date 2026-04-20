use rayon::prelude::*;
use std::time::Instant;

fn contains_p(word: &str) -> bool {
    // Simple work: does the word contain 'p' or 'P'?
    word.chars().any(|c| c == 'p' || c == 'P')
}

fn sequential<'a>(words: &'a[&'a str]) -> Vec<&'a str> {
    words.iter().copied().filter(|w| contains_p(w)).collect()
}

fn parallel<'a>(words: &'a [&'a str]) -> Vec<&'a str> {
    words.par_iter().copied().filter(|w| contains_p(w)).collect()
}

fn main() {
    let wiki_txt = "
        Parallel computing is a type of computation in which many calculations
        or processes are carried out simultaneously. Large problems can often
        be divided into smaller ones, which can then be solved at the same time.
        There are several forms of parallel computing: bit-level, instruction-level,
        data parallelism, task parallelism, and pipeline parallelism.
    ";

    // Make the dataset large enough so timing is meaningful.
    let big_text = wiki_txt.repeat(200_000);
    let words: Vec<&str> = big_text.split_whitespace().collect();

    println!("Total words: {}", words.len());

    let start_seq = Instant::now();
    let seq_result = sequential(&words);
    let seq_time = start_seq.elapsed();

    let start_par = Instant::now();
    let par_result = parallel(&words);
    let par_time = start_par.elapsed();

    assert_eq!(seq_result, par_result);

    println!("Sequential found {} words", seq_result.len());
    println!("Parallel   found {} words", par_result.len());
    println!("Sequential time: {:?}", seq_time);
    println!("Parallel time:   {:?}", par_time);

    if par_time < seq_time {
        println!(
            "Speedup: {:.2}x",
            seq_time.as_secs_f64() / par_time.as_secs_f64()
        );
    } else {
        println!(
            "Parallel was slower here: {:.2}x",
            par_time.as_secs_f64() / seq_time.as_secs_f64()
        );
    }

    println!(
        "First 20 matching words: {:?}",
        &seq_result[..seq_result.len().min(20)]
    );
}