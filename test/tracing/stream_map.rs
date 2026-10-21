#[cfg(test)]
mod stream_map_tests {
    use super::*;
    use crate::callsite::Callsite;
    use crate::field::Visit;
    use data_serialization::ser::Error as _;
    use std::prelude::v1::*;

    struct FailingMap {
        attempts: usize,
    }
    impl SerializeMap for FailingMap {
        type Ok = ();
        type Error = json_codec::Error;
        fn serialize_key<T: data_serialization::Serialize + ?Sized>(
            &mut self,
            _: &T,
        ) -> Result<(), Self::Error> {
            unreachable!()
        }
        fn serialize_value<T: data_serialization::Serialize + ?Sized>(
            &mut self,
            _: &T,
        ) -> Result<(), Self::Error> {
            unreachable!()
        }
        fn serialize_entry<
            K: data_serialization::Serialize + ?Sized,
            V: data_serialization::Serialize + ?Sized,
        >(
            &mut self,
            _: &K,
            _: &V,
        ) -> Result<(), Self::Error> {
            self.attempts += 1;
            Err(json_codec::Error::custom("intentional output failure"))
        }
        fn end(self) -> Result<(), Self::Error> {
            Ok(())
        }
    }

    #[test]
    fn first_serialization_failure_is_preserved_without_more_writes() {
        let callsite =
            crate::callsite! { name: "failure", kind: crate::metadata::Kind::EVENT, fields: value };
        let field = callsite.metadata().fields().field("value").unwrap();
        let mut map = FailingMap { attempts: 0 };
        let mut visitor = EventVisitor {
            map: &mut map,
            result: Ok(()),
        };
        visitor.record_i64(&field, 1);
        visitor.record_str(&field, "must not write");
        visitor.record_debug(&field, &false);
        assert!(visitor
            .result
            .unwrap_err()
            .to_string()
            .contains("intentional output failure"));
        assert_eq!(map.attempts, 1);
    }

    #[test]
    fn streaming_preserves_duplicate_keys_instead_of_silently_overwriting() {
        let callsite = crate::callsite! { name: "duplicates", kind: crate::metadata::Kind::EVENT, fields: value };
        let field = callsite.metadata().fields().field("value").unwrap();
        let mut bytes = Vec::new();
        let mut serializer = Serializer::new(&mut bytes);
        let mut map = serializer.serialize_map(None).unwrap();
        let mut visitor = EventVisitor {
            map: &mut map,
            result: Ok(()),
        };
        visitor.record_u64(&field, 1);
        visitor.record_u64(&field, 2);
        visitor.result.unwrap();
        map.end().unwrap();
        assert_eq!(bytes, br#"{"value":1,"value":2}"#);
    }
}
