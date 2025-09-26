//! This will eventually become benchmarks

#[cfg(test)]
mod tests {
    use ::magic_orb::*;
    use std::{
        sync::Arc, thread, time::{Duration, Instant}
    };

    #[test]
    fn bench_read_performance() {
        let buffer_size = 4096;
        let orb = Arc::new(MagicOrb::new(buffer_size, 0usize));
        let mut reads = 0;
        let start = Instant::now();

        while start.elapsed() < Duration::from_secs(1) {
            let _result = orb.get_contiguous();
            reads += 1;
        }

        eprintln!("\n--- Test results ---");
        eprintln!(
            "Reads within 1 sec (buffer size: {}): {}",
            buffer_size, reads
        );
    }

    #[test]
    fn bench_write_performance() {
        let buffer_size = 4096;
        let slice_size = 16;
        let orb = Arc::new(MagicOrb::new_default(buffer_size));
        let slice: Vec<usize> = (0..slice_size).collect();
        let mut writes = 0;
        let start = Instant::now();

        while start.elapsed() < Duration::from_secs(1) {
            orb.push_slice_overwrite(&slice);
            writes += 1;
        }

        eprintln!("\n--- Test results ---");
        eprintln!(
            "Slices writes of 16 bytes within 1 sec (buffer size: {}): {}",
            slice_size, writes
        );
    }

    #[test]
    fn bench_alternating_performance() {
        let buffer_size = 4096;
        let slice_size = 16;
        let orb = Arc::new(MagicOrb::new_default(buffer_size));
        let slice: Vec<usize> = (0..slice_size).collect();

        let orb_clone_read = Arc::clone(&orb);
        let read_handle = thread::spawn(move || {
            let mut reads = 0;
            let start = Instant::now();
            while start.elapsed() < Duration::from_secs(1) {
                let _result = orb_clone_read.get_contiguous();
                reads += 1;
            }
            reads
        });

        let orb_clone_write = Arc::clone(&orb);
        let write_handle = thread::spawn(move || {
            let mut writes = 0;
            let start = Instant::now();
            while start.elapsed() < Duration::from_secs(1) {
                orb_clone_write.push_slice_overwrite(&slice);
                writes += 1;
            }
            writes
        });

        let reads = read_handle.join().unwrap();
        let writes = write_handle.join().unwrap();

        eprintln!("\n--- Test results ---");
        eprintln!(
            "Reads - writes within 1 sec (buffer size: {}): Reads: {} Writes: {}",
            buffer_size, reads, writes
        );
    }
}