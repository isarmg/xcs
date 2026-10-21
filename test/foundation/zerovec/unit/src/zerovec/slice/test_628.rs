#[cfg(test)]
mod test {
    use super::*;
    use crate::zeroslice;

    #[test]
    fn test_split_first() {
        {
            // empty slice.
            assert_eq!(None, ZeroSlice::<u16>::new_empty().split_first());
        }
        {
            // single element slice
            const DATA: &ZeroSlice<u16> =
                zeroslice!(u16; <u16 as AsULE>::ULE::from_unsigned; [211]);
            assert_eq!((211, zeroslice![]), DATA.split_first().unwrap());
        }
        {
            // slice with many elements.
            const DATA: &ZeroSlice<u16> =
                zeroslice!(u16; <u16 as AsULE>::ULE::from_unsigned; [211, 281, 421, 32973]);
            const EXPECTED_VALUE: (u16, &ZeroSlice<u16>) = (
                211,
                zeroslice!(u16; <u16 as AsULE>::ULE::from_unsigned; [281, 421, 32973]),
            );

            assert_eq!(EXPECTED_VALUE, DATA.split_first().unwrap());
        }
    }
}
