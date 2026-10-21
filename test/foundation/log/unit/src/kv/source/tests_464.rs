#[cfg(test)]
mod tests {
    use crate::kv::value;

    use super::*;

    #[test]
    fn source_is_object_safe() {
        fn _check(_: &dyn Source) {}
    }

    #[test]
    fn visitor_is_object_safe() {
        fn _check(_: &dyn VisitSource) {}
    }

    #[test]
    fn count() {
        struct OnePair {
            key: &'static str,
            value: i32,
        }

        impl Source for OnePair {
            fn visit<'kvs>(&'kvs self, visitor: &mut dyn VisitSource<'kvs>) -> Result<(), Error> {
                visitor.visit_pair(self.key.to_key(), self.value.to_value())
            }
        }

        assert_eq!(1, Source::count(&("a", 1)));
        assert_eq!(2, Source::count(&[("a", 1), ("b", 2)] as &[_]));
        assert_eq!(0, Source::count(&None::<(&str, i32)>));
        assert_eq!(1, Source::count(&OnePair { key: "a", value: 1 }));
    }

    #[test]
    fn get() {
        let source = &[("a", 1), ("b", 2), ("a", 1)] as &[_];
        assert_eq!(
            value::inner::Token::I64(1),
            Source::get(source, Key::from_str("a")).unwrap().to_token()
        );
        assert_eq!(
            value::inner::Token::I64(2),
            Source::get(source, Key::from_str("b")).unwrap().to_token()
        );
        assert!(Source::get(&source, Key::from_str("c")).is_none());

        let source = None::<(&str, i32)>;
        assert!(Source::get(&source, Key::from_str("a")).is_none());
    }
}
