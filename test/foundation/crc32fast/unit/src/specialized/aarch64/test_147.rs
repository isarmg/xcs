#[cfg(test)]
mod test {
    quickcheck::quickcheck! {
        fn check_against_baseline(init: u32, chunks: Vec<(Vec<u8>, usize)>) -> bool {
            let mut baseline = super::super::super::baseline::State::new(init);
            let mut aarch64 = super::State::new(init).expect("not supported");
            for (chunk, mut offset) in chunks {
                // simulate random alignments by offsetting the slice by up to 15 bytes
                offset &= 0xF;
                if chunk.len() <= offset {
                    baseline.update(&chunk);
                    aarch64.update(&chunk);
                } else {
                    baseline.update(&chunk[offset..]);
                    aarch64.update(&chunk[offset..]);
                }
            }
            aarch64.finalize() == baseline.finalize()
        }
    }

    // Exercises the 3-way interleaved path across the sizes and alignments where it switches.
    #[test]
    fn check_large_inputs_against_baseline() {
        let mut data = vec![0u8; 8200];
        let mut s: u32 = 0x1234_5678;
        for byte in data.iter_mut() {
            s = s.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            *byte = (s >> 24) as u8;
        }

        for &len in &[
            0usize, 1, 7, 8, 15, 16, 63, 64, 127, 128, 1535, 1536, 1537, 4096, 8192, 8199,
        ] {
            for &offset in &[0usize, 1, 3, 7, 8, 15] {
                if offset + len > data.len() {
                    continue;
                }
                let slice = &data[offset..offset + len];
                let mut baseline = super::super::super::baseline::State::new(0);
                baseline.update(slice);
                let mut specialized = super::State::new(0).expect("not supported");
                specialized.update(slice);
                assert_eq!(
                    specialized.finalize(),
                    baseline.finalize(),
                    "mismatch for len={len} offset={offset}",
                );
            }
        }
    }
}
