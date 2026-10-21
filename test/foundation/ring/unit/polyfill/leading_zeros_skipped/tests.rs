#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_leading_zeroes_stripped() {
        static TEST_CASES: &[(&[u8], &[u8])] = &[
            (&[], &[]),
            (&[0], &[0]),
            (&[0, 1], &[1]),
            (&[0, 0, 1], &[1]),
            (&[0, 0, 0, 1], &[1]),
            (&[1, 0], &[1, 0]),
            (&[0, 1, 0], &[1, 0]),
        ];
        TEST_CASES.iter().copied().for_each(|(input, expected)| {
            let stripped = LeadingZerosStripped::new(input.iter().copied());
            super::super::test::assert_iterator(stripped, expected);
        });
    }
}
