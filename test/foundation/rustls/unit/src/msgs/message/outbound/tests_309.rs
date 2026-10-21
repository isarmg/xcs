#[cfg(test)]
mod tests {
    use std::{println, vec};

    use super::*;

    #[test]
    fn split_at_with_single_slice() {
        let owner: &[u8] = &[0, 1, 2, 3, 4, 5, 6, 7];
        let borrowed_payload = OutboundChunks::Single(owner);

        let (before, after) = borrowed_payload.split_at(6);
        println!("before:{before:?}\nafter:{after:?}");
        assert_eq!(before.to_vec(), &[0, 1, 2, 3, 4, 5]);
        assert_eq!(after.to_vec(), &[6, 7]);
    }

    #[test]
    fn split_at_with_multiple_slices() {
        let owner: Vec<&[u8]> = vec![&[0, 1, 2, 3], &[4, 5], &[6, 7, 8], &[9, 10, 11, 12]];
        let borrowed_payload = OutboundChunks::new(&owner);

        let (before, after) = borrowed_payload.split_at(3);
        println!("before:{before:?}\nafter:{after:?}");
        assert_eq!(before.to_vec(), &[0, 1, 2]);
        assert_eq!(after.to_vec(), &[3, 4, 5, 6, 7, 8, 9, 10, 11, 12]);

        let (before, after) = borrowed_payload.split_at(8);
        println!("before:{before:?}\nafter:{after:?}");
        assert_eq!(before.to_vec(), &[0, 1, 2, 3, 4, 5, 6, 7]);
        assert_eq!(after.to_vec(), &[8, 9, 10, 11, 12]);

        let (before, after) = borrowed_payload.split_at(11);
        println!("before:{before:?}\nafter:{after:?}");
        assert_eq!(before.to_vec(), &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
        assert_eq!(after.to_vec(), &[11, 12]);
    }

    #[test]
    fn split_out_of_bounds() {
        let owner: Vec<&[u8]> = vec![&[0, 1, 2, 3], &[4, 5], &[6, 7, 8], &[9, 10, 11, 12]];

        let single_payload = OutboundChunks::Single(owner[0]);
        let (before, after) = single_payload.split_at(17);
        println!("before:{before:?}\nafter:{after:?}");
        assert_eq!(before.to_vec(), &[0, 1, 2, 3]);
        assert!(after.is_empty());

        let multiple_payload = OutboundChunks::new(&owner);
        let (before, after) = multiple_payload.split_at(17);
        println!("before:{before:?}\nafter:{after:?}");
        assert_eq!(before.to_vec(), &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12]);
        assert!(after.is_empty());

        let empty_payload = OutboundChunks::new_empty();
        let (before, after) = empty_payload.split_at(17);
        println!("before:{before:?}\nafter:{after:?}");
        assert!(before.is_empty());
        assert!(after.is_empty());
    }

    #[test]
    fn empty_slices_mixed() {
        let owner: Vec<&[u8]> = vec![&[], &[], &[0], &[], &[1, 2], &[], &[3], &[4], &[], &[]];
        let mut borrowed_payload = OutboundChunks::new(&owner);
        let mut fragment_count = 0;
        let mut fragment;
        let expected_fragments: &[&[u8]] = &[&[0, 1], &[2, 3], &[4]];

        while !borrowed_payload.is_empty() {
            (fragment, borrowed_payload) = borrowed_payload.split_at(2);
            println!("{fragment:?}");
            assert_eq!(&expected_fragments[fragment_count], &fragment.to_vec());
            fragment_count += 1;
        }
        assert_eq!(fragment_count, expected_fragments.len());
    }

    #[test]
    fn exhaustive_splitting() {
        let owner: Vec<u8> = (0..127).collect();
        let slices = (0..7)
            .map(|i| &owner[((1 << i) - 1)..((1 << (i + 1)) - 1)])
            .collect::<Vec<_>>();
        let payload = OutboundChunks::new(&slices);

        assert_eq!(payload.to_vec(), owner);
        println!("{payload:#?}");

        for start in 0..128 {
            for end in start..128 {
                for mid in 0..(end - start) {
                    let witness = owner[start..end].split_at(mid);
                    let split_payload = payload
                        .split_at(end)
                        .0
                        .split_at(start)
                        .1
                        .split_at(mid);
                    assert_eq!(
                        witness.0,
                        split_payload.0.to_vec(),
                        "start: {start}, mid:{mid}, end:{end}"
                    );
                    assert_eq!(
                        witness.1,
                        split_payload.1.to_vec(),
                        "start: {start}, mid:{mid}, end:{end}"
                    );
                }
            }
        }
    }
}
