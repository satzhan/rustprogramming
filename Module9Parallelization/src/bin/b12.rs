use std::alloc::{alloc_zeroed, dealloc, handle_alloc_error, Layout};
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
use std::arch::is_x86_feature_detected;
#[cfg(target_arch = "x86")]
use std::arch::x86::*;
#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::*;
use std::hint::black_box;
use std::ptr::NonNull;
use std::slice;
use std::time::Instant;

const ALIGNMENT: usize = 32;
const LANES: usize = 8; // 8 f32 values in one 256-bit AVX register

struct AlignedF32Buffer {
    ptr: NonNull<f32>,
    len: usize,
    layout: Layout,
}

impl AlignedF32Buffer {
    fn new_zeroed(len: usize) -> Self {
        assert!(len > 0, "length must be > 0");
        let bytes = len
            .checked_mul(std::mem::size_of::<f32>())
            .expect("size overflow");
        let layout = Layout::from_size_align(bytes, ALIGNMENT).expect("invalid layout");

        unsafe {
            let raw = alloc_zeroed(layout) as *mut f32;
            let ptr = NonNull::new(raw).unwrap_or_else(|| handle_alloc_error(layout));
            Self { ptr, len, layout }
        }
    }

    fn new_filled(len: usize, value: f32) -> Self {
        let mut buf = Self::new_zeroed(len);
        buf.fill(value);
        buf
    }

    fn as_ptr(&self) -> *const f32 {
        self.ptr.as_ptr()
    }

    fn as_mut_ptr(&mut self) -> *mut f32 {
        self.ptr.as_ptr()
    }

    fn as_slice(&self) -> &[f32] {
        unsafe { slice::from_raw_parts(self.as_ptr(), self.len) }
    }

    fn as_mut_slice(&mut self) -> &mut [f32] {
        unsafe { slice::from_raw_parts_mut(self.as_mut_ptr(), self.len) }
    }

    fn fill(&mut self, value: f32) {
        for x in self.as_mut_slice() {
            *x = value;
        }
    }

    fn ptr_mod_32(&self) -> usize {
        (self.as_ptr() as usize) % ALIGNMENT
    }
}

impl Drop for AlignedF32Buffer {
    fn drop(&mut self) {
        unsafe {
            dealloc(self.ptr.as_ptr() as *mut u8, self.layout);
        }
    }
}

#[inline(never)]
fn regular_add(out: &mut [f32], a: &[f32], b: &[f32]) {
    assert_eq!(out.len(), a.len());
    assert_eq!(a.len(), b.len());

    for i in 0..a.len() {
        out[i] = a[i] + b[i];
    }
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
#[target_feature(enable = "avx")]
unsafe fn simd_add_unaligned(out: &mut [f32], a: &[f32], b: &[f32]) {
    assert_eq!(out.len(), a.len());
    assert_eq!(a.len(), b.len());

    let len = a.len();
    let chunks = len / LANES;

    for i in 0..chunks {
        let idx = i * LANES;
        let a_vec = _mm256_loadu_ps(a.as_ptr().add(idx));
        let b_vec = _mm256_loadu_ps(b.as_ptr().add(idx));
        let sum = _mm256_add_ps(a_vec, b_vec);
        _mm256_storeu_ps(out.as_mut_ptr().add(idx), sum);
    }

    for i in (chunks * LANES)..len {
        out[i] = a[i] + b[i];
    }
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
#[target_feature(enable = "avx")]
unsafe fn simd_add_aligned(out: &mut [f32], a: &[f32], b: &[f32]) {
    assert_eq!(out.len(), a.len());
    assert_eq!(a.len(), b.len());

    debug_assert_eq!((a.as_ptr() as usize) % ALIGNMENT, 0);
    debug_assert_eq!((b.as_ptr() as usize) % ALIGNMENT, 0);
    debug_assert_eq!((out.as_ptr() as usize) % ALIGNMENT, 0);

    let len = a.len();
    let chunks = len / LANES;

    for i in 0..chunks {
        let idx = i * LANES;
        let a_vec = _mm256_load_ps(a.as_ptr().add(idx));
        let b_vec = _mm256_load_ps(b.as_ptr().add(idx));
        let sum = _mm256_add_ps(a_vec, b_vec);
        _mm256_store_ps(out.as_mut_ptr().add(idx), sum);
    }

    for i in (chunks * LANES)..len {
        out[i] = a[i] + b[i];
    }
}

#[inline(never)]
fn sampled_sum(x: &[f32]) -> f32 {
    let step = (x.len() / 16).max(1);
    let mut acc = 0.0_f32;
    let mut i = 0;

    while i < x.len() {
        acc += x[i];
        i += step;
    }

    acc + x[x.len() - 1]
}

fn benchmark_ms<F>(iterations: usize, mut kernel: F) -> (f64, f32)
where
    F: FnMut(usize) -> f32,
{
    let mut total_ms = 0.0;
    let mut sink = 0.0_f32;

    for iter in 0..iterations {
        let start = Instant::now();
        sink += black_box(kernel(iter));
        total_ms += start.elapsed().as_secs_f64() * 1000.0;
    }

    (total_ms / iterations as f64, black_box(sink))
}

fn main() {
    let size = 1024 * 1024;
    let iterations = 20000;

    println!("Vector Addition Benchmark (corrected)");
    println!("Array size : {} elements", size);
    println!("Iterations : {}\n", iterations);

    // 1) Regular benchmark with ordinary Vec<T>
    let a_regular = vec![1.0_f32; size];
    let b_regular = vec![2.0_f32; size];
    let mut out_regular = vec![0.0_f32; size];

    // 2) Intentionally unaligned benchmark.
    // We allocate a 32-byte aligned buffer and then start at element 1,
    // which shifts the address by 4 bytes and makes it unaligned for AVX loads.
    let a_unaligned_full = AlignedF32Buffer::new_filled(size + 1, 1.0);
    let b_unaligned_full = AlignedF32Buffer::new_filled(size + 1, 2.0);
    let mut out_unaligned_full = AlignedF32Buffer::new_zeroed(size + 1);

    let a_unaligned = &a_unaligned_full.as_slice()[1..=size];
    let b_unaligned = &b_unaligned_full.as_slice()[1..=size];
    let out_unaligned = &mut out_unaligned_full.as_mut_slice()[1..=size];

    // 3) Truly aligned benchmark.
    let a_aligned_full = AlignedF32Buffer::new_filled(size, 1.0);
    let b_aligned_full = AlignedF32Buffer::new_filled(size, 2.0);
    let mut out_aligned_full = AlignedF32Buffer::new_zeroed(size);

    let a_aligned = a_aligned_full.as_slice();
    let b_aligned = b_aligned_full.as_slice();
    let out_aligned = out_aligned_full.as_mut_slice();

    let (regular_ms, regular_sink) = benchmark_ms(iterations, |iter| {
        let a = black_box(a_regular.as_slice());
        let b = black_box(b_regular.as_slice());
        let out = black_box(out_regular.as_mut_slice());
        regular_add(out, a, b);
        black_box(out[iter % out.len()] + sampled_sum(out))
    });

    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    let avx_available = is_x86_feature_detected!("avx");
    #[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
    let avx_available = false;

    if !avx_available {
        println!("AVX is not available on this machine, so only the regular version can run.");
        println!("Regular Addition : {:.3} ms", regular_ms);
        println!("Regular sink     : {:.1}", regular_sink);
        return;
    }

    let (simd_ms, simd_sink) = benchmark_ms(iterations, |iter| {
        let a = black_box(a_unaligned);
        let b = black_box(b_unaligned);
        let out = black_box(&mut *out_unaligned);
        unsafe { simd_add_unaligned(out, a, b) };
        black_box(out[iter % out.len()] + sampled_sum(out))
    });

    let (aligned_ms, aligned_sink) = benchmark_ms(iterations, |iter| {
        let a = black_box(a_aligned);
        let b = black_box(b_aligned);
        let out = black_box(&mut *out_aligned);
        unsafe { simd_add_aligned(out, a, b) };
        black_box(out[iter % out.len()] + sampled_sum(out))
    });

    println!("1. Regular Addition     : {:.3} ms", regular_ms);
    println!(
        "2. SIMD Addition        : {:.3} ms ({:.2}x speedup vs regular)",
        simd_ms,
        regular_ms / simd_ms
    );
    println!(
        "3. Aligned SIMD         : {:.3} ms ({:.2}x speedup vs regular)",
        aligned_ms,
        regular_ms / aligned_ms
    );

    println!("\nResult checks:");
    println!("Regular sink : {:.1}", regular_sink);
    println!("SIMD sink    : {:.1}", simd_sink);
    println!("Aligned sink : {:.1}", aligned_sink);
    println!("Regular sum  : {:.1}", out_regular.iter().sum::<f32>());
    println!("SIMD sum     : {:.1}", out_unaligned.iter().sum::<f32>());
    println!("Aligned sum  : {:.1}", out_aligned.iter().sum::<f32>());

    println!("\nPointer alignment (mod 32):");
    println!("Regular input       : {}", (a_regular.as_ptr() as usize) % ALIGNMENT);
    println!("SIMD input          : {}", (a_unaligned.as_ptr() as usize) % ALIGNMENT);
    println!("Aligned input       : {}", (a_aligned.as_ptr() as usize) % ALIGNMENT);
    println!("Unaligned base      : {}", a_unaligned_full.ptr_mod_32());
    println!("Aligned buffer base : {}", a_aligned_full.ptr_mod_32());
}
