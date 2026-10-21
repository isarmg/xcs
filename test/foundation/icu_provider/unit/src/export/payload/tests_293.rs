#[cfg(test)]
mod tests {
    use super::*;
    use crate::hello_world::*;

    #[test]
    fn test_compare_with_dyn() {
        let payload1: DataPayload<HelloWorldV1> = DataPayload::from_owned(HelloWorld {
            message: "abc".into(),
        });
        let payload2: DataPayload<HelloWorldV1> = DataPayload::from_owned(HelloWorld {
            message: "abc".into(),
        });
        let payload3: DataPayload<HelloWorldV1> = DataPayload::from_owned(HelloWorld {
            message: "def".into(),
        });

        assert!(payload1.eq_dyn(&payload2));
        assert!(payload2.eq_dyn(&payload1));

        assert!(!payload1.eq_dyn(&payload3));
        assert!(!payload3.eq_dyn(&payload1));
    }

    #[test]
    fn test_export_marker_partial_eq() {
        let payload1: DataPayload<ExportMarker> =
            UpcastDataPayload::upcast(DataPayload::<HelloWorldV1>::from_owned(HelloWorld {
                message: "abc".into(),
            }));
        let payload2: DataPayload<ExportMarker> =
            UpcastDataPayload::upcast(DataPayload::<HelloWorldV1>::from_owned(HelloWorld {
                message: "abc".into(),
            }));
        let payload3: DataPayload<ExportMarker> =
            UpcastDataPayload::upcast(DataPayload::<HelloWorldV1>::from_owned(HelloWorld {
                message: "def".into(),
            }));

        assert_eq!(payload1, payload2);
        assert_eq!(payload2, payload1);
        assert_ne!(payload1, payload3);
        assert_ne!(payload3, payload1);
    }
}
