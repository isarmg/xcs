#[cfg(test)]
mod test {
    use alloc::{borrow::ToOwned, boxed::Box, string::String};
    use std::format;

    use super::*;
    use crate::metadata::{Kind, Level, Metadata};

    // Make sure TEST_CALLSITE_* have non-zero size, so they can't be located at the same address.
    struct TestCallsite1 {
        _unused: u8,
    }
    static TEST_CALLSITE_1: TestCallsite1 = TestCallsite1 { _unused: 0 };
    static TEST_META_1: Metadata<'static> = metadata! {
        name: "field_test1",
        target: module_path!(),
        level: Level::INFO,
        fields: &["foo", "bar", "baz"],
        callsite: &TEST_CALLSITE_1,
        kind: Kind::SPAN,
    };

    impl crate::callsite::Callsite for TestCallsite1 {
        fn set_interest(&self, _: crate::subscriber::Interest) {
            unimplemented!()
        }

        fn metadata(&self) -> &Metadata<'_> {
            &TEST_META_1
        }
    }

    struct TestCallsite2 {
        _unused: u8,
    }
    static TEST_CALLSITE_2: TestCallsite2 = TestCallsite2 { _unused: 0 };
    static TEST_META_2: Metadata<'static> = metadata! {
        name: "field_test2",
        target: module_path!(),
        level: Level::INFO,
        fields: &["foo", "bar", "baz"],
        callsite: &TEST_CALLSITE_2,
        kind: Kind::SPAN,
    };

    impl crate::callsite::Callsite for TestCallsite2 {
        fn set_interest(&self, _: crate::subscriber::Interest) {
            unimplemented!()
        }

        fn metadata(&self) -> &Metadata<'_> {
            &TEST_META_2
        }
    }

    #[test]
    fn value_set_with_no_values_is_empty() {
        let fields = TEST_META_1.fields();
        let values = &[
            (&fields.field("foo").unwrap(), None),
            (&fields.field("bar").unwrap(), None),
            (&fields.field("baz").unwrap(), None),
        ];
        let valueset = fields.value_set(values);
        assert!(valueset.is_empty());
    }

    #[test]
    fn index_of_field_in_fieldset_is_correct() {
        let fields = TEST_META_1.fields();
        let foo = fields.field("foo").unwrap();
        assert_eq!(foo.index(), 0);
        let bar = fields.field("bar").unwrap();
        assert_eq!(bar.index(), 1);
        let baz = fields.field("baz").unwrap();
        assert_eq!(baz.index(), 2);
    }

    #[test]
    fn empty_value_set_is_empty() {
        let fields = TEST_META_1.fields();
        let valueset = fields.value_set(&[]);
        assert!(valueset.is_empty());
    }

    #[test]
    fn value_sets_with_fields_from_other_callsites_are_empty() {
        let fields = TEST_META_1.fields();
        let values = &[
            (&fields.field("foo").unwrap(), Some(&1 as &dyn Value)),
            (&fields.field("bar").unwrap(), Some(&2 as &dyn Value)),
            (&fields.field("baz").unwrap(), Some(&3 as &dyn Value)),
        ];
        let valueset = TEST_META_2.fields().value_set(values);
        assert!(valueset.is_empty())
    }

    #[test]
    fn sparse_value_sets_are_not_empty() {
        let fields = TEST_META_1.fields();
        let values = &[
            (&fields.field("foo").unwrap(), None),
            (&fields.field("bar").unwrap(), Some(&57 as &dyn Value)),
            (&fields.field("baz").unwrap(), None),
        ];
        let valueset = fields.value_set(values);
        assert!(!valueset.is_empty());
    }

    #[test]
    fn fields_from_other_callsets_are_skipped() {
        let fields = TEST_META_1.fields();
        let values = &[
            (&fields.field("foo").unwrap(), None),
            (
                &TEST_META_2.fields().field("bar").unwrap(),
                Some(&57 as &dyn Value),
            ),
            (&fields.field("baz").unwrap(), None),
        ];

        struct MyVisitor;
        impl Visit for MyVisitor {
            fn record_debug(&mut self, field: &Field, _: &dyn fmt::Debug) {
                assert_eq!(field.callsite(), TEST_META_1.callsite())
            }
        }
        let valueset = fields.value_set(values);
        valueset.record(&mut MyVisitor);
    }

    #[test]
    fn empty_fields_are_skipped() {
        let fields = TEST_META_1.fields();
        let values = &[
            (&fields.field("foo").unwrap(), Some(&Empty as &dyn Value)),
            (&fields.field("bar").unwrap(), Some(&57 as &dyn Value)),
            (&fields.field("baz").unwrap(), Some(&Empty as &dyn Value)),
        ];

        struct MyVisitor;
        impl Visit for MyVisitor {
            fn record_debug(&mut self, field: &Field, _: &dyn fmt::Debug) {
                assert_eq!(field.name(), "bar")
            }
        }
        let valueset = fields.value_set(values);
        valueset.record(&mut MyVisitor);
    }

    #[test]
    fn record_debug_fn() {
        let fields = TEST_META_1.fields();
        let values = &[
            (&fields.field("foo").unwrap(), Some(&1 as &dyn Value)),
            (&fields.field("bar").unwrap(), Some(&2 as &dyn Value)),
            (&fields.field("baz").unwrap(), Some(&3 as &dyn Value)),
        ];
        let valueset = fields.value_set(values);
        let mut result = String::new();
        valueset.record(&mut |_: &Field, value: &dyn fmt::Debug| {
            use core::fmt::Write;
            write!(&mut result, "{:?}", value).unwrap();
        });
        assert_eq!(result, "123".to_owned());
    }

    #[test]
    #[cfg(feature = "std")]
    fn record_error() {
        let fields = TEST_META_1.fields();
        let err: Box<dyn std::error::Error + Send + Sync + 'static> =
            std::io::Error::new(std::io::ErrorKind::Other, "lol").into();
        let values = &[
            (&fields.field("foo").unwrap(), Some(&err as &dyn Value)),
            (&fields.field("bar").unwrap(), Some(&Empty as &dyn Value)),
            (&fields.field("baz").unwrap(), Some(&Empty as &dyn Value)),
        ];
        let valueset = fields.value_set(values);
        let mut result = String::new();
        valueset.record(&mut |_: &Field, value: &dyn fmt::Debug| {
            use core::fmt::Write;
            write!(&mut result, "{:?}", value).unwrap();
        });
        assert_eq!(result, format!("{}", err));
    }

    #[test]
    fn record_bytes() {
        let fields = TEST_META_1.fields();
        let first = &b"abc"[..];
        let second: &[u8] = &[192, 255, 238];
        let values = &[
            (&fields.field("foo").unwrap(), Some(&first as &dyn Value)),
            (&fields.field("bar").unwrap(), Some(&" " as &dyn Value)),
            (&fields.field("baz").unwrap(), Some(&second as &dyn Value)),
        ];
        let valueset = fields.value_set(values);
        let mut result = String::new();
        valueset.record(&mut |_: &Field, value: &dyn fmt::Debug| {
            use core::fmt::Write;
            write!(&mut result, "{:?}", value).unwrap();
        });
        assert_eq!(result, format!("{}", r#"[61 62 63]" "[c0 ff ee]"#));
    }
}
