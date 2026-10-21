#[cfg(test)]
mod tests {
    use crate::{Array, sizes::U3};
    type A = Array<u8, U3>;

    #[test]
    #[cfg(feature = "alloc")]
    fn expecting() {
        use alloc::string::ToString;
        let err = serde_json::from_str::<A>("true").unwrap_err();
        assert!(err.to_string().contains("expected an array of length 3"));
    }

    #[test]
    fn round_trip() {
        let example: A = Array([1, 2, 3]);
        let s = serde_json::to_string(&example).unwrap();
        let deserialized: A = serde_json::from_str(&s).unwrap();
        assert_eq!(example, deserialized);
    }
}
