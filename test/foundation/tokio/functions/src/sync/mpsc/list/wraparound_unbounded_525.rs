#[test]
    fn wraparound_unbounded() {
        let (tx, mut rx) = unbounded_channel_from_index(0usize.wrapping_sub(2 * BLOCK_CAP));

        for i in 0..4 * BLOCK_CAP {
            tx.send(i).unwrap();
            assert_eq!(rx.len(), i + 1);
            assert!(!rx.is_empty());
        }

        for i in 0..4 * BLOCK_CAP {
            assert_eq!(rx.try_recv().unwrap(), i);
            assert_eq!(rx.len(), 4 * BLOCK_CAP - 1 - i);
        }
        assert!(rx.is_empty());
    }
