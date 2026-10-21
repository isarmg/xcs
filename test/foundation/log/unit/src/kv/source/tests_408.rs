#[cfg(test)]
    mod tests {
        use crate::kv::value;

        use super::*;

        #[test]
        fn count() {
            assert_eq!(1, Source::count(&Box::new(("a", 1))));
            assert_eq!(2, Source::count(&vec![("a", 1), ("b", 2)]));
        }

        #[test]
        fn get() {
            let source = vec![("a", 1), ("b", 2), ("a", 1)];
            assert_eq!(
                value::inner::Token::I64(1),
                Source::get(&source, Key::from_str("a")).unwrap().to_token()
            );

            let source = Box::new(None::<(&str, i32)>);
            assert!(Source::get(&source, Key::from_str("a")).is_none());
        }

        #[test]
        fn hash_map() {
            let mut map = HashMap::new();
            map.insert("a", 1);
            map.insert("b", 2);

            assert_eq!(2, Source::count(&map));
            assert_eq!(
                value::inner::Token::I64(1),
                Source::get(&map, Key::from_str("a")).unwrap().to_token()
            );
        }

        #[test]
        fn btree_map() {
            let mut map = BTreeMap::new();
            map.insert("a", 1);
            map.insert("b", 2);

            assert_eq!(2, Source::count(&map));
            assert_eq!(
                value::inner::Token::I64(1),
                Source::get(&map, Key::from_str("a")).unwrap().to_token()
            );
        }
    }
