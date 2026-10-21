#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_fmt_should_display_quotes_for_strings() {
        let s: SharedValue<str> = SharedValue::Borrowed("hello");
        assert_eq!(format!("{s:?}"), "\"hello\"");
        let s: SharedValue<str> = SharedValue::Counted(Arc::from("hello"));
        assert_eq!(format!("{s:?}"), "\"hello\"");
    }

    #[test]
    fn partial_eq_should_compare_str_to_str() {
        let s: SharedValue<str> = SharedValue::Borrowed("hello");
        assert_eq!(s, "hello");
        assert_eq!("hello", s);
        assert_eq!(s, String::from("hello"));
        assert_eq!(String::from("hello"), s);
        assert_eq!(s, Cow::from("hello"));
        assert_eq!(Cow::from("hello"), s);
    }

    #[test]
    fn partial_eq_should_compare_slice_to_slice() {
        let s: SharedValue<[i32]> = SharedValue::Borrowed([1, 2, 3].as_slice());
        assert_eq!(s, [1, 2, 3].as_slice());
        assert_eq!([1, 2, 3].as_slice(), s);
        assert_eq!(s, vec![1, 2, 3]);
        assert_eq!(vec![1, 2, 3], s);
        assert_eq!(s, Cow::<'_, [i32]>::Borrowed(&[1, 2, 3]));
        assert_eq!(Cow::<'_, [i32]>::Borrowed(&[1, 2, 3]), s);
    }

    #[test]
    fn add_should_concatenate_strings() {
        let s: SharedValue<str> = SharedValue::Borrowed("hello");
        assert_eq!(s.clone() + " world", "hello world");
        assert_eq!(s.clone() + Cow::from(" world"), "hello world");
        assert_eq!(s + SharedValue::from(" world"), "hello world");
    }

    #[test]
    fn as_str_should_return_a_str() {
        let s: SharedValue<str> = SharedValue::Borrowed("hello");
        assert_eq!(s.as_str(), "hello");
        let s: SharedValue<str> = SharedValue::Counted(Arc::from("hello"));
        assert_eq!(s.as_str(), "hello");
    }

    #[test]
    fn as_slice_should_return_a_slice() {
        let s: SharedValue<[i32]> = SharedValue::Borrowed([1, 2, 3].as_slice());
        assert_eq!(s.as_slice(), [1, 2, 3].as_slice());
        let s: SharedValue<[i32]> = SharedValue::Counted(Arc::from([1, 2, 3]));
        assert_eq!(s.as_slice(), [1, 2, 3].as_slice());
    }

    #[test]
    fn default_for_str_should_return_an_empty_string() {
        let s: SharedValue<str> = Default::default();
        assert!(s.is_empty());
    }

    #[test]
    fn default_for_slice_should_return_an_empty_slice() {
        let s: SharedValue<[i32]> = Default::default();
        assert!(s.is_empty());
    }

    #[test]
    fn default_for_any_option_should_return_none() {
        let s: SharedValue<Option<i32>> = Default::default();
        assert!(s.is_none());
    }

    #[test]
    fn cloned_owned_string_should_make_counted_str() {
        let s: SharedValue<str> = SharedValue::Owned(String::from("hello"));
        assert!(s.clone().is_counted());
    }

    #[test]
    fn cloned_borrowed_str_should_make_borrowed_str() {
        let s: SharedValue<str> = SharedValue::Borrowed("hello");
        assert!(s.clone().is_borrowed());
    }

    #[test]
    fn cloned_counted_str_should_make_counted_str() {
        let s: SharedValue<str> = SharedValue::Counted(Arc::from("hello"));
        assert!(s.clone().is_counted());
    }

    #[test]
    fn cloned_inplace_owned_string_should_make_counted_str_and_become_counted()
    {
        let mut s: SharedValue<str> = SharedValue::Owned(String::from("hello"));
        assert!(s.clone_inplace().is_counted());
        assert!(s.is_counted());
    }

    #[test]
    fn cloned_inplace_borrowed_str_should_make_borrowed_str_and_remain_borrowed(
    ) {
        let mut s: SharedValue<str> = SharedValue::Borrowed("hello");
        assert!(s.clone_inplace().is_borrowed());
        assert!(s.is_borrowed());
    }

    #[test]
    fn cloned_inplace_counted_str_should_make_counted_str_and_remain_counted() {
        let mut s: SharedValue<str> = SharedValue::Counted(Arc::from("hello"));
        assert!(s.clone_inplace().is_counted());
        assert!(s.is_counted());
    }

    #[test]
    fn serialization_works() {
        let s = json_codec::to_string(&SharedValue::Borrowed("foo"))
            .expect("should serialize string");
        assert_eq!(s, "\"foo\"");
    }

    #[test]
    fn deserialization_works() {
        let s: SharedValue<str> = json_codec::from_str("\"bar\"")
            .expect("should deserialize from string");
        assert_eq!(s, SharedValue::from(String::from("bar")));
    }
}
