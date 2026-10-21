#[cfg(test)]
mod tests {
    use super::*;
    use crate::codepointinvlist::CodePointInversionList;
    use core::{char, convert::TryFrom};

    #[test]
    fn test_try_from_range() {
        let check: Vec<char> = CodePointInversionList::try_from('A'..'B')
            .unwrap()
            .iter_chars()
            .collect();
        assert_eq!(vec!['A'], check);
    }

    #[test]
    fn test_try_from_range_error() {
        let check = CodePointInversionList::try_from('A'..'A');
        assert!(matches!(check, Err(RangeError(65, 65))));
    }

    #[test]
    fn test_try_from_range_inclusive() {
        let check: Vec<char> = CodePointInversionList::try_from('A'..='A')
            .unwrap()
            .iter_chars()
            .collect();
        assert_eq!(vec!['A'], check);
    }

    #[test]
    fn test_try_from_range_inclusive_err() {
        let check = CodePointInversionList::try_from('B'..'A');
        assert!(matches!(check, Err(RangeError(66, 65))));
    }

    #[test]
    fn test_try_from_range_from() {
        let uset = CodePointInversionList::try_from('A'..).unwrap();
        let check: usize = uset.size();
        let expected: usize = (char::MAX as usize) + 1 - 65;
        assert_eq!(expected, check);
    }

    #[test]
    fn test_try_from_range_to() {
        let uset = CodePointInversionList::try_from(..'A').unwrap();
        let check: usize = uset.size();
        let expected: usize = 65;
        assert_eq!(expected, check);
    }

    #[test]
    fn test_try_from_range_to_err() {
        let check = CodePointInversionList::try_from(..(0x0 as char));
        assert!(matches!(check, Err(RangeError(0, 0))));
    }

    #[test]
    fn test_try_from_range_to_inclusive() {
        let uset = CodePointInversionList::try_from(..='A').unwrap();
        let check: usize = uset.size();
        let expected: usize = 66;
        assert_eq!(expected, check);
    }

    #[test]
    fn test_try_from_range_full() {
        let uset = CodePointInversionList::try_from(..).unwrap();
        let check: usize = uset.size();
        let expected: usize = (char::MAX as usize) + 1;
        assert_eq!(expected, check);
    }

    #[test]
    fn test_from_range_iterator() {
        let ranges = [
            0..=0x3FFF,
            0x4000..=0x7FFF,
            0x8000..=0xBFFF,
            0xC000..=0xFFFF,
        ];
        let expected =
            CodePointInversionList::try_from_u32_inversion_list_slice(&[0x0, 0x1_0000]).unwrap();
        let actual = CodePointInversionList::from_iter(ranges);
        assert_eq!(expected, actual);
    }
}
