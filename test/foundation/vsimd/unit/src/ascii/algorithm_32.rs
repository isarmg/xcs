#[cfg(test)]
mod algorithm {
    use crate::algorithm::*;

    #[test]
    #[ignore]
    fn convert_case() {
        let convert = |c: u8, shift: u8| {
            let x1 = c.wrapping_sub(shift + 0x80);
            let x2 = i8_lt(x1 as i8, -0x80 + 26);
            let x3 = x2 & 0x20;
            c ^ x3
        };
        let to_upper = |c: u8| convert(c, b'a');
        let to_lower = |c: u8| convert(c, b'A');

        print_fn_table(|c| c.is_ascii_lowercase(), to_upper);
        print_fn_table(|c| c.is_ascii_uppercase(), to_lower);

        for c in 0..=255u8 {
            assert_eq!(to_upper(c), c.to_ascii_uppercase());
            assert_eq!(to_lower(c), c.to_ascii_lowercase());
        }
    }
}
