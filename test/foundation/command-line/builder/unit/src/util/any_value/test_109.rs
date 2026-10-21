#[cfg(test)]
mod test {
    #[test]
    #[cfg(debug_assertions)]
    fn debug_impl() {
        use super::*;

        assert_eq!(format!("{:?}", AnyValue::new(5)), "AnyValue { inner: i32 }");
    }

    #[test]
    fn eq_to_type_id() {
        use super::*;

        let any_value_id = AnyValueId::of::<i32>();
        let type_id = std::any::TypeId::of::<i32>();
        assert_eq!(any_value_id, type_id);
    }
}
