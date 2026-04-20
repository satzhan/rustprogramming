use std::arch::x86_64::*;

fn main() {
    let size = 16;
    let mut base_memory = vec![0.0_f32; size + 8];

    unsafe {
        let ptr = base_memory.as_mut_ptr();
        let align_offset = ptr.align_offset(32);
        
        let aligned_ptr = ptr.add(align_offset);
        let unaligned_ptr = aligned_ptr.add(1);

        println!("Aligned address:   {:p}", aligned_ptr);
        println!("Unaligned address: {:p}", unaligned_ptr);
        println!("Executing hardware load...");

        let _crash_vector = _mm256_load_ps(unaligned_ptr);

        println!("You will never see this line printed.");
    }
}