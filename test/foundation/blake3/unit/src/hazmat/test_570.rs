#[cfg(test)]
mod test {
    use super::*;

    #[test]
    #[should_panic]
    fn test_empty_subtree_should_panic() {
        Hasher::new().finalize_non_root();
    }

    #[test]
    #[should_panic]
    fn test_unaligned_offset_should_panic() {
        Hasher::new().set_input_offset(1);
    }

    #[test]
    #[should_panic]
    fn test_hasher_already_accepted_input_should_panic() {
        Hasher::new().update(b"x").set_input_offset(0);
    }

    #[test]
    #[should_panic]
    fn test_too_much_input_should_panic() {
        Hasher::new()
            .set_input_offset(CHUNK_LEN as u64)
            .update(&[0; CHUNK_LEN + 1]);
    }

    #[test]
    #[should_panic]
    fn test_set_input_offset_cant_finalize() {
        Hasher::new().set_input_offset(CHUNK_LEN as u64).finalize();
    }

    #[test]
    #[should_panic]
    fn test_set_input_offset_cant_finalize_xof() {
        Hasher::new()
            .set_input_offset(CHUNK_LEN as u64)
            .finalize_xof();
    }

    #[test]
    fn test_grouped_hash() {
        const MAX_CHUNKS: usize = (crate::test::TEST_CASES_MAX + 1) / CHUNK_LEN;
        let mut input_buf = [0; crate::test::TEST_CASES_MAX];
        crate::test::paint_test_input(&mut input_buf);
        for subtree_chunks in [1, 2, 4, 8, 16, 32] {
            #[cfg(feature = "std")]
            dbg!(subtree_chunks);
            let subtree_len = subtree_chunks * CHUNK_LEN;
            for &case in crate::test::TEST_CASES {
                if case <= subtree_len {
                    continue;
                }
                #[cfg(feature = "std")]
                dbg!(case);
                let input = &input_buf[..case];
                let expected_hash = crate::hash(input);

                // Collect all the group chaining values.
                let mut chaining_values = arrayvec::ArrayVec::<ChainingValue, MAX_CHUNKS>::new();
                let mut subtree_offset = 0;
                while subtree_offset < input.len() {
                    let take = core::cmp::min(subtree_len, input.len() - subtree_offset);
                    let subtree_input = &input[subtree_offset..][..take];
                    let subtree_cv = Hasher::new()
                        .set_input_offset(subtree_offset as u64)
                        .update(subtree_input)
                        .finalize_non_root();
                    chaining_values.push(subtree_cv);
                    subtree_offset += take;
                }

                // Compress all the chaining_values together, layer by layer.
                assert!(chaining_values.len() >= 2);
                while chaining_values.len() > 2 {
                    let n = chaining_values.len();
                    // Merge each side-by-side pair in place, overwriting the front half of the
                    // array with the merged results. This moves us "up one level" in the tree.
                    for i in 0..(n / 2) {
                        chaining_values[i] = merge_subtrees_non_root(
                            &chaining_values[2 * i],
                            &chaining_values[2 * i + 1],
                            Mode::Hash,
                        );
                    }
                    // If there's an odd CV out, it moves up.
                    if n % 2 == 1 {
                        chaining_values[n / 2] = chaining_values[n - 1];
                    }
                    chaining_values.truncate(n / 2 + n % 2);
                }
                assert_eq!(chaining_values.len(), 2);
                let root_hash =
                    merge_subtrees_root(&chaining_values[0], &chaining_values[1], Mode::Hash);
                assert_eq!(expected_hash, root_hash);
            }
        }
    }

    #[test]
    fn test_keyed_hash_xof() {
        let group0 = &[42; 4096];
        let group1 = &[43; 4095];
        let mut input = [0; 8191];
        input[..4096].copy_from_slice(group0);
        input[4096..].copy_from_slice(group1);
        let key = &[44; 32];

        let mut expected_output = [0; 100];
        Hasher::new_keyed(&key)
            .update(&input)
            .finalize_xof()
            .fill(&mut expected_output);

        let mut hazmat_output = [0; 100];
        let left = Hasher::new_keyed(key).update(group0).finalize_non_root();
        let right = Hasher::new_keyed(key)
            .set_input_offset(group0.len() as u64)
            .update(group1)
            .finalize_non_root();
        merge_subtrees_root_xof(&left, &right, Mode::KeyedHash(&key)).fill(&mut hazmat_output);
        assert_eq!(expected_output, hazmat_output);
    }

    #[test]
    fn test_derive_key() {
        let context = "foo";
        let mut input = [0; 1025];
        crate::test::paint_test_input(&mut input);
        let expected = crate::derive_key(context, &input);

        let cx_key = hash_derive_key_context(context);
        let left = Hasher::new_from_context_key(&cx_key)
            .update(&input[..1024])
            .finalize_non_root();
        let right = Hasher::new_from_context_key(&cx_key)
            .set_input_offset(1024)
            .update(&input[1024..])
            .finalize_non_root();
        let derived_key = merge_subtrees_root(&left, &right, Mode::DeriveKeyMaterial(&cx_key)).0;
        assert_eq!(expected, derived_key);
    }
}
