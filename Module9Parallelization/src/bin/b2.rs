use std::arch::x86_64::*;

fn main() {
    // We allocate 12 elements. We only need 8 for an AVX register, 
    // but the extra padding allows us to slide our view forward.
    let mut data = vec![0.0_f32; 12];
    
    // Fill with recognizable data: 100, 101, 102...
    for i in 0..12 {
        data[i] = 100.0 + i as f32;
    }

    unsafe {
        let base_ptr = data.as_mut_ptr();
        let align_offset = base_ptr.align_offset(32);
        
        // This pointer is mathematically perfect. Address ends in 0x00, 0x20, 0x40, etc.
        let aligned_ptr = base_ptr.add(align_offset);
        
        // We shift forward by exactly one f32 (4 bytes).
        // Address ends in 0x04, 0x24, 0x44, etc.
        let unaligned_ptr = aligned_ptr.add(1);

        println!("=== Memory Map ===");
        println!("Index | Value | Hex Address | Aligned (32-byte)?");
        for i in 0..10 {
            let current_ptr = aligned_ptr.add(i);
            let addr = current_ptr as usize;
            let is_aligned = addr % 32 == 0;
            println!("  [{}] | {} | {:#010x}  | {}", 
                     i, *current_ptr, addr, is_aligned);
        }

        println!("\n=== The Almost Crash (Safe Unaligned Load) ===");
        println!("Fetching 8 floats starting at {:#010x}...", unaligned_ptr as usize);
        
        // We use the 'u' (unaligned) intrinsic.
        let safe_vector = _mm256_loadu_ps(unaligned_ptr);
        
        // We extract the data to prove we grabbed exactly what we wanted (101 to 108).
        let mut extracted = [0.0_f32; 8];
        _mm256_storeu_ps(extracted.as_mut_ptr(), safe_vector);
        println!("Successfully loaded: {:?}", extracted);

        println!("\n=== The Hard Crash (Unsafe Aligned Load) ===");
        println!("Lying to the CPU: telling it {:#010x} is aligned...", unaligned_ptr as usize);
        
        // This is where you die.
        let _fatal_vector = _mm256_load_ps(unaligned_ptr);
        
        println!("You will never reach this line.");
    }
}

// Index:      [0]     [1]     [2]     [3]     [4]     [5]     [6]     [7]     [8]     [9]
// Value:     100.0   101.0   102.0   103.0   104.0   105.0   106.0   107.0   108.0   109.0
// Address:   0x00    0x04    0x08    0x0C    0x10    0x14    0x18    0x1C    0x20    0x24