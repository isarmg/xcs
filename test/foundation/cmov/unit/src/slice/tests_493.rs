#[cfg(test)]
mod tests {
    #[test]
    fn cmovnz_remainder() {
        // - Test endianness handling on non-64-bit platforms
        // - Test handling of odd length slices on 64-bit platforms
        #[cfg(not(target_pointer_width = "64"))]
        const A_U16: [u16; 2] = [0xAAAA, 0xBBBB];
        #[cfg(target_pointer_width = "64")]
        const A_U16: [u16; 3] = [0xAAAA, 0xBBBB, 0xCCCC];

        #[cfg(not(target_pointer_width = "64"))]
        const B_U16: [u16; 2] = [0x10, 0xFFFF];
        #[cfg(target_pointer_width = "64")]
        const B_U16: [u16; 3] = [0x10, 0x10, 0xFFFF];

        let mut out = A_U16;

        super::cmovnz_remainder(&mut out, &B_U16, 0);
        assert_eq!(A_U16, out);

        super::cmovnz_remainder(&mut out, &B_U16, 1);
        assert_eq!(B_U16, out);
    }

    #[test]
    fn slice_to_word() {
        assert_eq!(0xAABBCC, super::slice_to_word(&[0xCCu8, 0xBB, 0xAA]));
        assert_eq!(0xAAAABBBB, super::slice_to_word(&[0xBBBBu16, 0xAAAA]));

        #[cfg(target_pointer_width = "64")]
        assert_eq!(
            0xAAAABBBBCCCC,
            super::slice_to_word(&[0xCCCCu16, 0xBBBB, 0xAAAA])
        );
    }

    #[test]
    fn word_to_slice() {
        let mut out = [0u8; 3];
        super::word_to_slice(0xAABBCC, &mut out);
        assert_eq!(&[0xCC, 0xBB, 0xAA], &out);

        let mut out = [0u16; 2];
        super::word_to_slice(0xAAAABBBB, &mut out);
        assert_eq!(&[0xBBBB, 0xAAAA], &out);

        #[cfg(target_pointer_width = "64")]
        {
            let mut out = [0u16; 3];
            super::word_to_slice(0xAAAABBBBCCCC, &mut out);
            assert_eq!(&[0xCCCC, 0xBBBB, 0xAAAA], &out);
        }
    }
}
