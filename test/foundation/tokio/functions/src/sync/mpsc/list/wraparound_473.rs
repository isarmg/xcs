#[test]
    #[cfg(not(target_family = "wasm"))]
    fn wraparound() {
        use super::*;

        let (tx, mut rx) = channel_from_index(0usize.wrapping_sub(2 * BLOCK_CAP));
        let head = rx.free_head;

        for i in 0..BLOCK_CAP {
            tx.push(i);
            assert!(matches!(rx.pop(&tx), Some(block::Read::Value(v)) if v == i));
        }

        // Simulate a slow sender claiming the first slot of the second block
        // while still holding a pointer to the first block.
        let slow_slot = tx.tail_position.fetch_add(1, Acquire);
        let slow_tail = AtomicPtr::new(tx.block_tail.load(Acquire));

        // Fill the rest of the second block (wrapping `tail_position` to 0),
        // then push one item into the third block to retire the first block.
        for i in 1..BLOCK_CAP {
            tx.push(i);
        }
        assert_eq!(rx.len(&tx), BLOCK_CAP);
        tx.push(BLOCK_CAP);

        // Advancing `rx` to the second block must not reclaim the first block yet,
        // because `rx.index` has not reached the wrapped `required_index` (1).
        std::thread::scope(|s| {
            s.spawn(|| unsafe {
                let slow_tail = &*slow_tail.load(Relaxed);
                let slow_block = slow_tail.load_next(Acquire).unwrap();
                assert!(slow_block
                    .as_ref()
                    .is_at_index(block::start_index(slow_slot)));
            });
            assert!(rx.pop(&tx).is_none());
        });
        assert_eq!(rx.free_head, head);

        unsafe {
            let slow_tail = &*slow_tail.load(Relaxed);
            let slow_block = slow_tail.load_next(Acquire).unwrap();
            slow_block.as_ref().write(slow_slot, 0);
        }

        for i in 0..=BLOCK_CAP {
            assert!(matches!(rx.pop(&tx), Some(block::Read::Value(v)) if v == i));
        }
        unsafe { rx.free_blocks() };
    }
