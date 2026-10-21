#[cfg(test)]
mod tests {
    // Spot check up to a given limit
    const TEST_LIMIT: u8 = 128;

    macro_rules! masknz_test {
        ( $($name:ident : $uint:ident),+ ) => {
            $(
                #[test]
                fn $name() {
                    assert_eq!(masknz!(0: $uint), 0);

                    // Test lower values
                    for i in 1..=$uint::from(TEST_LIMIT) {
                        assert_eq!(masknz!(i: $uint), $uint::MAX);
                    }

                    // Test upper values
                    for i in ($uint::MAX - $uint::from(TEST_LIMIT))..=$uint::MAX {
                        assert_eq!(masknz!(i: $uint), $uint::MAX);
                    }
                }
            )+
        }
    }

    // Ensure the macro works with any types we might use it with (we only use u8, u32, and u64)
    masknz_test!(
        masknz_u8: u8,
        masknz_u16: u16,
        masknz_u32: u32,
        masknz_u64: u64,
        masknz_u128: u128,
        masknz_usize: usize
    );
}
