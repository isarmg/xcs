#[cfg(test)]
mod tests {
    use super::{PotentialCodePoint, deconstruct_range, is_valid_zv};
    use zerovec::ZeroVec;

    fn make_zv(slice: &[u32]) -> ZeroVec<'_, PotentialCodePoint> {
        slice
            .iter()
            .copied()
            .map(PotentialCodePoint::from_u24)
            .collect()
    }
    #[test]
    fn test_is_valid_zv() {
        let check = make_zv(&[0x2, 0x3, 0x4, 0x5]);
        assert!(is_valid_zv(&check));
    }

    #[test]
    fn test_is_valid_zv_empty() {
        let check = make_zv(&[]);
        assert!(is_valid_zv(&check));
    }

    #[test]
    fn test_is_valid_zv_overlapping() {
        let check = make_zv(&[0x2, 0x5, 0x4, 0x6]);
        assert!(!is_valid_zv(&check));
    }

    #[test]
    fn test_is_valid_zv_out_of_order() {
        let check = make_zv(&[0x5, 0x4, 0x5, 0x6, 0x7]);
        assert!(!is_valid_zv(&check));
    }

    #[test]
    fn test_is_valid_zv_duplicate() {
        let check = make_zv(&[0x1, 0x2, 0x3, 0x3, 0x5]);
        assert!(!is_valid_zv(&check));
    }

    #[test]
    fn test_is_valid_zv_odd() {
        let check = make_zv(&[0x1, 0x2, 0x3, 0x4, 0x5]);
        assert!(!is_valid_zv(&check));
    }

    #[test]
    fn test_is_valid_zv_out_of_range() {
        let check = make_zv(&[0x1, 0x2, 0x3, 0x4, (char::MAX as u32) + 1]);
        assert!(!is_valid_zv(&check));
    }

    // deconstruct_range

    #[test]
    fn test_deconstruct_range() {
        let expected = (0x41, 0x45);
        let check = deconstruct_range('A'..'E'); // Range
        assert_eq!(check, expected);
        let check = deconstruct_range('A'..='D'); // Range Inclusive
        assert_eq!(check, expected);
        let check = deconstruct_range('A'..); // Range From
        assert_eq!(check, (0x41, (char::MAX as u32) + 1));
        let check = deconstruct_range(..'A'); // Range To
        assert_eq!(check, (0x0, 0x41));
        let check = deconstruct_range(..='A'); // Range To Inclusive
        assert_eq!(check, (0x0, 0x42));
        let check = deconstruct_range::<char>(..); // Range Full
        assert_eq!(check, (0x0, (char::MAX as u32) + 1));
    }
}
