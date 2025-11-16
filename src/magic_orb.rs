#[cfg(feature = "not_nightly")]
use crate::sync_unsafe_cell::SyncUnsafeCell;
#[cfg(not(feature = "not_nightly"))]
use std::cell::SyncUnsafeCell;

use std::{
    cmp::min,
    fmt::Debug,
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
    len: Arc<AtomicUsize>,
    capacity: Arc<AtomicUsize>,
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
            len: Arc::new(AtomicUsize::new(size)),
            capacity: Arc::new(AtomicUsize::new(size)),
        }
    }
}

impl<T: Clone + Send> From<Vec<T>> for MagicOrb<T> {
    fn from(value: Vec<T>) -> Self {
        if value.is_empty() {
            panic!("Can't crate MagicOrb with buffer size 0");
        }
        MagicOrb {
            capacity: Arc::new(AtomicUsize::new(value.len())),
            len: Arc::new(AtomicUsize::new(value.len())),
            buf: Arc::new(SyncUnsafeCell::new(value)),
            write: Arc::new(AtomicUsize::new(0)),
            lock: Arc::new(AtomicBool::new(false)),
        }
    }
}

impl<T: Clone + Send + Debug> MagicOrb<T> {
    pub fn new(size: usize, default_val: T) -> Self {
        if size == 0 {
            panic!("Can't crate MagicOrb with buffer size 0");
        }
        MagicOrb {
            buf: Arc::new(SyncUnsafeCell::new(vec![default_val; size])),
            write: Arc::new(AtomicUsize::new(0)),
            lock: Arc::new(AtomicBool::new(false)),
            len: Arc::new(AtomicUsize::new(size)),
            capacity: Arc::new(AtomicUsize::new(size)),
        }
    }

    pub fn push_slice_overwrite(&self, data: &[T]) {
        let occupied = self.capacity.load(Ordering::Acquire);
        let data = if data.len() > occupied {
            &data[data.len() - occupied..]
        } else {
            data
        };

        self.take_lock();
        {
            // SAFETY: Lock prevents aliasing &mut T
            // Guarantees required: should be between self.take_lock() and self.return_lock()
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
                self.write.store(data.len() - first_len, Ordering::Relaxed);
            }

            let capacity = self.capacity();

            _ = self
                .len
                .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |cur_val| {
                    if cur_val < capacity {
                        Some(min(cur_val + data.len(), capacity))
                    } else {
                        None
                    }
                });
        }
        self.return_lock();
    }

    pub fn get_contiguous(&self) -> Vec<T> {
        let capacity = self.capacity();
        let mut ret = Vec::with_capacity(capacity);

        self.take_lock();
        if self.is_empty() {
            self.return_lock();
            return ret;
        }

        let vacant_amount = {
            // SAFETY: Lock prevents aliasing &mut T.
            // Guarantees required: should be between self.take_lock() and self.return_lock()
            let buf = unsafe { self.buf.get().as_mut().unwrap() };
            let write = self.write.load(Ordering::Relaxed);
            let vacant_amount = capacity - self.len();
            ret.extend_from_slice(&buf[write..]);
            ret.extend_from_slice(&buf[..write]);
            vacant_amount
        };
        self.return_lock();

        if vacant_amount > 0 {
            ret = ret.split_off(vacant_amount);
        }

        ret
    }

    pub fn pop_back(&self) {
        self.take_lock();
        {
            if self
                .len
                .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |cur_val| {
                    if cur_val == 0 {
                        None
                    } else {
                        Some(cur_val - 1)
                    }
                })
                .is_ok()
            {
                let max_len = self.capacity();
                _ = self
                    .write
                    .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |idx| {
                        Some((idx + max_len - 1) % max_len)
                    });
            }
        }
        self.return_lock();
    }

    pub fn capacity(&self) -> usize {
        self.capacity.load(Ordering::Relaxed)
    }

    pub fn len(&self) -> usize {
        self.len.load(Ordering::Relaxed)
    }

    pub fn is_empty(&self) -> bool {
        if self.len() == 0 {
            return true;
        }
        false
    }

    fn take_lock(&self) {
        while self
            .lock
            .compare_exchange(false, true, Ordering::Release, Ordering::Acquire)
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
    use std::thread;

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
    fn test_pop_front_from_full_orb() {
        let orb = MagicOrb::from(vec![1, 2, 3, 4, 5]);
        assert_eq!(orb.len(), 5);
        orb.pop_back();
        assert_eq!(orb.len(), 4);
        let contents = orb.get_contiguous();
        assert_eq!(contents, vec![1, 2, 3, 4]);
    }

    #[test]
    fn test_pop_front_until_empty() {
        let orb = MagicOrb::from(vec![1, 2, 3]);
        assert_eq!(orb.len(), 3);
        orb.pop_back();
        assert_eq!(orb.len(), 2);
        let contents = orb.get_contiguous();
        assert_eq!(contents, vec![1, 2]);

        orb.pop_back();
        assert_eq!(orb.len(), 1);
        let contents = orb.get_contiguous();
        assert_eq!(contents, vec![1]);

        orb.pop_back();
        assert_eq!(orb.len(), 0);
        assert!(orb.is_empty());
        let contents = orb.get_contiguous();
        assert!(contents.is_empty());
    }

    #[test]
    fn test_pop_front_from_empty_orb() {
        let orb = MagicOrb::new(5, 0);
        assert_eq!(orb.len(), 5);
        for _ in 0..5 {
            orb.pop_back();
        }
        assert_eq!(orb.len(), 0);

        orb.pop_back();
        assert_eq!(orb.len(), 0);
        assert!(orb.is_empty());
    }

    #[test]
    fn test_pop_front_after_push_and_overwrite() {
        let orb = MagicOrb::new_default(4);
        orb.push_slice_overwrite(&[1, 2, 3, 4, 5, 6]);
        assert_eq!(orb.len(), 4);
        let contents = orb.get_contiguous();
        assert_eq!(contents, vec![3, 4, 5, 6]);

        orb.pop_back();
        orb.pop_back();
        orb.push_slice_overwrite(&[7]);
        assert_eq!(orb.len(), 3);
        let contents = orb.get_contiguous();
        assert_eq!(contents, vec![3, 4, 7]);

        orb.push_slice_overwrite(&[9, 10]);
        assert_eq!(orb.get_contiguous(), vec![4, 7, 9, 10]);
        orb.pop_back();
        assert_eq!(orb.len(), 3);
        let contents = orb.get_contiguous();
        assert_eq!(contents, vec![4, 7, 9]);
    }
}
