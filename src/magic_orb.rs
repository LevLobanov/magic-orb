#[cfg(feature = "not_nightly")]
use crate::sync_unsafe_cell::SyncUnsafeCell;
#[cfg(not(feature = "not_nightly"))]
use std::cell::SyncUnsafeCell;

use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
};

#[derive(Debug, Clone)]
pub struct MagicOrb<T>
where
    T: Clone + Send,
{
    buf: Arc<SyncUnsafeCell<Vec<T>>>,
    write: Arc<AtomicUsize>,
    lock: Arc<AtomicBool>,
    len: usize,
}

impl<T: Default + Clone + Send> MagicOrb<T> {
    pub fn new_default(size: usize) -> Self {
        if size == 0 {
            panic!("Can't crate MagicOrb with buffer size 0");
        }
        MagicOrb {
            buf: Arc::new(SyncUnsafeCell::new(vec![T::default(); size])),
            write: Arc::new(AtomicUsize::new(0)),
            lock: Arc::new(AtomicBool::new(false)),
            len: size,
        }
    }
}

impl<T: Clone + Send> From<Vec<T>> for MagicOrb<T> {
    fn from(value: Vec<T>) -> Self {
        if value.is_empty() {
            panic!("Can't crate MagicOrb with buffer size 0");
        }
        MagicOrb {
            len: value.len(),
            buf: Arc::new(SyncUnsafeCell::new(value)),
            write: Arc::new(AtomicUsize::new(0)),
            lock: Arc::new(AtomicBool::new(false)),
        }
    }
}

impl<T: Clone + Send> MagicOrb<T> {
    pub fn new(size: usize, default_val: T) -> Self {
        if size == 0 {
            panic!("Can't crate MagicOrb with buffer size 0");
        }
        MagicOrb {
            buf: Arc::new(SyncUnsafeCell::new(vec![default_val; size])),
            write: Arc::new(AtomicUsize::new(0)),
            lock: Arc::new(AtomicBool::new(false)),
            len: size,
        }
    }

    pub fn push_slice_overwrite(&self, data: &[T]) {
        let occupied = self.len;
        let data = if data.len() > occupied {
            &data[data.len() - occupied..]
        } else {
            data
        };

        self.take_lock();
        {
            // SAFETY: Lock prevents aliasing &mut T
            // Guarantees requred: should be between self.take_lock() and self.return_lock()
            let buf = unsafe { self.buf.get().as_mut().unwrap() };
            let write = self.write.load(Ordering::Relaxed);

            if data.len() + write <= occupied {
                buf[write..write + data.len()].clone_from_slice(data);
                self.write
                    .store((write + data.len()) % occupied, Ordering::Relaxed);
            } else {
                let first_len = occupied - write;
                buf[write..].clone_from_slice(&data[..first_len]);
                buf[..data.len() - first_len].clone_from_slice(&data[first_len..]);
            }
        }
        self.return_lock();
    }

    pub fn get_contiguous(&self) -> Vec<T> {
        let mut ret = Vec::with_capacity(self.len);

        self.take_lock();
        {
            // SAFETY: Lock prevents aliasing &mut T.
            // Guarantees requred: should be between self.take_lock() and self.return_lock()
            let buf = unsafe { self.buf.get().as_mut().unwrap() };
            let write = self.write.load(Ordering::Relaxed);
            ret.extend_from_slice(&buf[write..]);
            ret.extend_from_slice(&buf[..write]);
        }
        self.return_lock();

        ret
    }

    fn take_lock(&self) {
        while self
            .lock
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::Acquire)
            .is_err()
        {
            std::hint::spin_loop();
        }
    }

    fn return_lock(&self) {
        self.lock.store(false, Ordering::Release);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{thread, time::{Duration, Instant}};

    #[test]
    fn test_new_default() {
        let orb: MagicOrb<i32> = MagicOrb::new_default(5);
        let result = orb.get_contiguous();
        assert_eq!(result, vec![0, 0, 0, 0, 0]);
    }

    #[test]
    fn test_new_with_value() {
        let orb = MagicOrb::new(5, 7);
        let result = orb.get_contiguous();
        assert_eq!(result, vec![7, 7, 7, 7, 7]);
    }

    #[test]
    fn test_from_vec() {
        let data = vec![1, 2, 3, 4, 5];
        let orb = MagicOrb::from(data);
        let result = orb.get_contiguous();
        assert_eq!(result, vec![1, 2, 3, 4, 5]);
    }

    #[test]
    fn test_push_non_wrapping() {
        let orb = MagicOrb::from(vec![1, 2, 3, 4, 5]);
        orb.push_slice_overwrite(&[6, 7]);
        let result = orb.get_contiguous();
        assert_eq!(result, vec![3, 4, 5, 6, 7]);
    }

    #[test]
    fn test_push_wrapping() {
        let orb = MagicOrb::from(vec![1, 2, 3, 4, 5]);
        orb.push_slice_overwrite(&[6, 7, 8, 9, 10]);
        let result = orb.get_contiguous();
        assert_eq!(result, vec![6, 7, 8, 9, 10]);
    }

    #[test]
    fn test_push_wrapping_partial_1() {
        let orb = MagicOrb::from(vec![1, 2, 3, 4, 5]);
        orb.push_slice_overwrite(&[6, 7, 8, 9]);
        let result = orb.get_contiguous();
        assert_eq!(result, vec![5, 6, 7, 8, 9]);
    }

    #[test]
    fn test_push_larger_than_buffer() {
        let orb = MagicOrb::from(vec![1, 2, 3, 4, 5]);
        orb.push_slice_overwrite(&[10, 11, 12, 13, 14, 15, 16]);
        let result = orb.get_contiguous();
        assert_eq!(result, vec![12, 13, 14, 15, 16]);
    }

    #[test]
    fn test_multithreaded_push() {
        let orb = Arc::new(MagicOrb::new(1000, 0usize));
        let mut handles = vec![];

        for i in 0..10 {
            let orb_clone = Arc::clone(&orb);
            handles.push(thread::spawn(move || {
                let data: Vec<usize> = (i * 100..(i + 1) * 100).collect();
                orb_clone.push_slice_overwrite(&data);
            }));
        }

        for handle in handles {
            handle.join().unwrap();
        }

        let result = orb.get_contiguous();

        assert_eq!(result.iter().sum::<usize>(), 500 * 999usize);
    }

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

        eprintln!("\n--- Результаты теста ---");
        eprintln!("Количество чтений за 1 секунду (размер буфера {}): {}", buffer_size, reads);
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

        eprintln!("\n--- Результаты теста ---");
        eprintln!("Количество записей (срез размером {}) за 1 секунду: {}", slice_size, writes);
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
        
        eprintln!("\n--- Результаты теста ---");
        eprintln!("Чередование операций за 1 секунду:");
        eprintln!("Количество чтений: {}", reads);
        eprintln!("Количество записей: {}", writes);
    }
}