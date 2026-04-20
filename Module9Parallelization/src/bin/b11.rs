use std::arch::x86_64::*;
use std::time::Instant;
use std::hint::black_box;

fn add_regular(a: &[f32], b: &[f32], result: &mut [f32]) {
    for i in 0..a.len() {
        result[i] = a[i] + b[i];
    }
}

#[target_feature(enable = "avx")]
unsafe fn add_avx_unaligned(a: &[f32], b: &[f32], result: &mut [f32]) {
    let chunks = a.len() / 8;
    let a_ptr = a.as_ptr();
    let b_ptr = b.as_ptr();
    let res_ptr = result.as_mut_ptr();

    for i in 0..chunks {
        let idx = i * 8;
        let a_vec = _mm256_loadu_ps(a_ptr.add(idx));
        let b_vec = _mm256_loadu_ps(b_ptr.add(idx));
        let sum = _mm256_add_ps(a_vec, b_vec);
        _mm256_storeu_ps(res_ptr.add(idx), sum);
    }

    for i in (chunks * 8)..a.len() {
        result[i] = a[i] + b[i];
    }
}

#[target_feature(enable = "avx")]
unsafe fn add_avx_aligned(a: &[f32], b: &[f32], result: &mut [f32]) {
    let chunks = a.len() / 8;
    let a_ptr = a.as_ptr();
    let b_ptr = b.as_ptr();
    let res_ptr = result.as_mut_ptr();

    for i in 0..chunks {
        let idx = i * 8;
        let a_vec = _mm256_load_ps(a_ptr.add(idx));
        let b_vec = _mm256_load_ps(b_ptr.add(idx));
        let sum = _mm256_add_ps(a_vec, b_vec);
        _mm256_store_ps(res_ptr.add(idx), sum);
    }

    for i in (chunks * 8)..a.len() {
        result[i] = a[i] + b[i];
    }
}

fn main() {
    if !is_x86_feature_detected!("avx") {
        println!("Hardware Error: AVX is not supported on this CPU.");
        return;
    }

    let size = 1_000_000;
    let iterations = 1000;

    println!("Initializing memory for {} elements...", size);

    let mut backing_a = vec![1.0_f32; size + 8];
    let mut backing_b = vec![2.0_f32; size + 8];
    
    let mut backing_result_aligned = vec![0.0_f32; size + 8];
    let mut backing_result_unaligned = vec![0.0_f32; size + 8];

    let off_a = backing_a.as_ptr().align_offset(32);
    let off_b = backing_b.as_ptr().align_offset(32);
    let off_res_aligned = backing_result_aligned.as_ptr().align_offset(32);
    let off_res_unaligned = backing_result_unaligned.as_ptr().align_offset(32);

    let aligned_a = &backing_a[off_a .. off_a + size];
    let aligned_b = &backing_b[off_b .. off_b + size];
    
    let mut aligned_res = &mut backing_result_aligned[off_res_aligned .. off_res_aligned + size];

    let unaligned_a = &backing_a[off_a + 1 .. off_a + 1 + size];
    let unaligned_b = &backing_b[off_b + 1 .. off_b + 1 + size];
    
    let mut unaligned_res = &mut backing_result_unaligned[off_res_unaligned + 1 .. off_res_unaligned + 1 + size];

    println!("Running benchmarks ({} iterations)...\n", iterations);

    let start = Instant::now();
    for _ in 0..iterations {
        add_regular(black_box(aligned_a), black_box(aligned_b), black_box(&mut aligned_res));
    }
    let regular_time = start.elapsed();
    println!("1. Regular Scalar Addition : {:.2?}", regular_time);

    let start = Instant::now();
    for _ in 0..iterations {
        unsafe {
            add_avx_unaligned(black_box(unaligned_a), black_box(unaligned_b), black_box(&mut unaligned_res));
        }
    }
    let unaligned_time = start.elapsed();
    println!("2. Unaligned AVX Addition  : {:.2?}", unaligned_time);

    let start = Instant::now();
    for _ in 0..iterations {
        unsafe {
            add_avx_aligned(black_box(aligned_a), black_box(aligned_b), black_box(&mut aligned_res));
        }
    }
    let aligned_time = start.elapsed();
    println!("3. Aligned AVX Addition    : {:.2?}", aligned_time);
}