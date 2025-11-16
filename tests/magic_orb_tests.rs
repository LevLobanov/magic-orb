use magic_orb::MagicOrb;

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
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
