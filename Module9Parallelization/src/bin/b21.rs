use std::arch::x86_64::*;

fn main() {
    let mut data = vec![0.0_f32; 12];
    
    // Fill with recognizable data: 100, 101, 102...
    for i in 0..12 {
        data[i] = 100.0 + i as f32;
    }

    unsafe {
        let base_ptr = data.as_mut_ptr();
        let align_offset = base_ptr.align_offset(32);
        let aligned_f32_ptr = base_ptr.add(align_offset);
        
        // 1. Drop down to raw bytes (u8)
        let byte_ptr = aligned_f32_ptr as *mut u8;
        
        // 2. Shift exactly 1 byte forward
        let shifted_byte_ptr = byte_ptr.add(1);
        
        // 3. Cast back up to f32. This is highly unsafe.
        let one_byte_off_ptr = shifted_byte_ptr as *mut f32;

        println!("Aligned Address:     {:#010x}", aligned_f32_ptr as usize);
        println!("1-Byte Off Address:  {:#010x}", one_byte_off_ptr as usize);
        println!("Aligned Value:       {}", *aligned_f32_ptr);
        
        // Read the mangled scalar value
        println!("1-Byte Off Value:    {}\n", *one_byte_off_ptr);

        println!("Executing Unaligned AVX Load on the corrupted pointer...");
        let safe_vector = _mm256_loadu_ps(one_byte_off_ptr);
        
        let mut extracted = [0.0_f32; 8];
        _mm256_storeu_ps(extracted.as_mut_ptr(), safe_vector);
        
        println!("Successfully loaded, but look at the data:");
        for (i, val) in extracted.iter().enumerate() {
            println!("  AVX Slot [{}]: {}", i, val);
        }
    }
}