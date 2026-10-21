#[cfg(test)]
mod test {
    use super::*;

    use std::ops::RangeBounds;

    #[test]
    fn from_fixed() {
        let range: ValueRange = 5.into();
        assert_eq!(range.start_bound(), std::ops::Bound::Included(&5));
        assert_eq!(range.end_bound(), std::ops::Bound::Included(&5));
        assert!(range.is_fixed());
        assert!(range.is_multiple());
        assert_eq!(range.num_values(), Some(5));
        assert!(range.takes_values());
    }

    #[test]
    fn from_fixed_empty() {
        let range: ValueRange = 0.into();
        assert_eq!(range.start_bound(), std::ops::Bound::Included(&0));
        assert_eq!(range.end_bound(), std::ops::Bound::Included(&0));
        assert!(range.is_fixed());
        assert!(!range.is_multiple());
        assert_eq!(range.num_values(), Some(0));
        assert!(!range.takes_values());
    }

    #[test]
    fn from_range() {
        let range: ValueRange = (5..10).into();
        assert_eq!(range.start_bound(), std::ops::Bound::Included(&5));
        assert_eq!(range.end_bound(), std::ops::Bound::Included(&9));
        assert!(!range.is_fixed());
        assert!(range.is_multiple());
        assert_eq!(range.num_values(), None);
        assert!(range.takes_values());
    }

    #[test]
    fn from_range_inclusive() {
        let range: ValueRange = (5..=10).into();
        assert_eq!(range.start_bound(), std::ops::Bound::Included(&5));
        assert_eq!(range.end_bound(), std::ops::Bound::Included(&10));
        assert!(!range.is_fixed());
        assert!(range.is_multiple());
        assert_eq!(range.num_values(), None);
        assert!(range.takes_values());
    }

    #[test]
    fn from_range_full() {
        let range: ValueRange = (..).into();
        assert_eq!(range.start_bound(), std::ops::Bound::Included(&0));
        assert_eq!(range.end_bound(), std::ops::Bound::Included(&usize::MAX));
        assert!(!range.is_fixed());
        assert!(range.is_multiple());
        assert_eq!(range.num_values(), None);
        assert!(range.takes_values());
    }

    #[test]
    fn from_range_from() {
        let range: ValueRange = (5..).into();
        assert_eq!(range.start_bound(), std::ops::Bound::Included(&5));
        assert_eq!(range.end_bound(), std::ops::Bound::Included(&usize::MAX));
        assert!(!range.is_fixed());
        assert!(range.is_multiple());
        assert_eq!(range.num_values(), None);
        assert!(range.takes_values());
    }

    #[test]
    fn from_range_to() {
        let range: ValueRange = (..10).into();
        assert_eq!(range.start_bound(), std::ops::Bound::Included(&0));
        assert_eq!(range.end_bound(), std::ops::Bound::Included(&9));
        assert!(!range.is_fixed());
        assert!(range.is_multiple());
        assert_eq!(range.num_values(), None);
        assert!(range.takes_values());
    }

    #[test]
    fn from_range_to_inclusive() {
        let range: ValueRange = (..=10).into();
        assert_eq!(range.start_bound(), std::ops::Bound::Included(&0));
        assert_eq!(range.end_bound(), std::ops::Bound::Included(&10));
        assert!(!range.is_fixed());
        assert!(range.is_multiple());
        assert_eq!(range.num_values(), None);
        assert!(range.takes_values());
    }
}
