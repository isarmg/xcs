#[cfg(test)]
mod tests {
    // This test demonstrates a SWAR technique I tried in the sparse transition
    // code inside of 'next_state'. Namely, sparse transitions work by
    // iterating over u32 chunks, with each chunk containing up to 4 classes
    // corresponding to 4 transitions. This SWAR technique lets us find a
    // matching transition without converting the u32 to a [u8; 4].
    //
    // It turned out to be a little slower unfortunately, which isn't too
    // surprising, since this is likely a throughput oriented optimization.
    // Loop unrolling doesn't really help us because the vast majority of
    // states have very few transitions.
    //
    // Anyway, this code was a little tricky to write, so I converted it to a
    // test in case someone figures out how to use it more effectively than
    // I could.
    //
    // (This also only works on little endian. So big endian would need to be
    // accounted for if we ever decided to use this I think.)
    #[cfg(target_endian = "little")]
    #[test]
    fn swar() {
        use super::*;

        fn has_zero_byte(x: u32) -> u32 {
            const LO_U32: u32 = 0x01010101;
            const HI_U32: u32 = 0x80808080;

            x.wrapping_sub(LO_U32) & !x & HI_U32
        }

        fn broadcast(b: u8) -> u32 {
            (u32::from(b)) * (u32::MAX / 255)
        }

        fn index_of(x: u32) -> usize {
            let o =
                (((x - 1) & 0x01010101).wrapping_mul(0x01010101) >> 24) - 1;
            o.as_usize()
        }

        let bytes: [u8; 4] = [b'1', b'A', b'a', b'z'];
        let chunk = u32::from_ne_bytes(bytes);

        let needle = broadcast(b'1');
        assert_eq!(0, index_of(has_zero_byte(needle ^ chunk)));
        let needle = broadcast(b'A');
        assert_eq!(1, index_of(has_zero_byte(needle ^ chunk)));
        let needle = broadcast(b'a');
        assert_eq!(2, index_of(has_zero_byte(needle ^ chunk)));
        let needle = broadcast(b'z');
        assert_eq!(3, index_of(has_zero_byte(needle ^ chunk)));
    }
}
