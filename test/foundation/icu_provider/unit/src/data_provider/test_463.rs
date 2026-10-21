#[cfg(test)]
mod test {

    use super::*;
    use crate::hello_world::*;
    use alloc::borrow::Cow;
    use alloc::string::String;
    use core::fmt::Debug;
    use serde::{Deserialize, Serialize};

    // This tests DataProvider borrow semantics with a dummy data provider based on a
    // JSON string. It also exercises most of the data provider code paths.

    /// A data struct serialization-compatible with [`HelloWorld`] used for testing mismatched types
    #[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq, yoke::Yokeable)]
    pub struct HelloAlt {
        message: String,
    }

    data_marker!(HelloAltMarkerV1, HelloAlt);

    #[derive(Deserialize, Debug, Clone, Default, PartialEq)]
    struct HelloCombined<'data> {
        #[serde(borrow)]
        pub hello_v1: HelloWorld<'data>,
        pub hello_alt: HelloAlt,
    }

    /// A [`DataProvider`] that owns its data, returning an Rc-variant [`DataPayload`].
    /// Supports only [`HelloWorldV1`]. Uses `impl_dynamic_data_provider!()`.
    #[derive(Debug)]
    struct DataWarehouse {
        hello_v1: HelloWorld<'static>,
        hello_alt: HelloAlt,
    }

    impl DataProvider<HelloWorldV1> for DataWarehouse {
        fn load(&self, _: DataRequest) -> Result<DataResponse<HelloWorldV1>, DataError> {
            Ok(DataResponse {
                metadata: DataResponseMetadata::default(),
                payload: DataPayload::from_owned(self.hello_v1.clone()),
            })
        }
    }

    /// A [`DataProvider`] that supports both [`HelloWorldV1`] and [`HelloAltV1`].
    #[derive(Debug)]
    struct DataProvider2 {
        data: DataWarehouse,
    }

    impl From<DataWarehouse> for DataProvider2 {
        fn from(warehouse: DataWarehouse) -> Self {
            DataProvider2 { data: warehouse }
        }
    }

    impl DataProvider<HelloWorldV1> for DataProvider2 {
        fn load(&self, _: DataRequest) -> Result<DataResponse<HelloWorldV1>, DataError> {
            Ok(DataResponse {
                metadata: DataResponseMetadata::default(),
                payload: DataPayload::from_owned(self.data.hello_v1.clone()),
            })
        }
    }

    impl DataProvider<HelloAltMarkerV1> for DataProvider2 {
        fn load(&self, _: DataRequest) -> Result<DataResponse<HelloAltMarkerV1>, DataError> {
            Ok(DataResponse {
                metadata: DataResponseMetadata::default(),
                payload: DataPayload::from_owned(self.data.hello_alt.clone()),
            })
        }
    }

    const DATA: &str = r#"{
        "hello_v1": {
            "message": "Hello "
        },
        "hello_alt": {
            "message": "Hello Alt"
        }
    }"#;

    fn get_warehouse(data: &'static str) -> DataWarehouse {
        let data: HelloCombined = serde_json::from_str(data).expect("Well-formed data");
        DataWarehouse {
            hello_v1: data.hello_v1,
            hello_alt: data.hello_alt,
        }
    }

    fn get_payload_v1<P: DataProvider<HelloWorldV1> + ?Sized>(
        provider: &P,
    ) -> Result<DataPayload<HelloWorldV1>, DataError> {
        provider.load(Default::default()).map(|r| r.payload)
    }

    fn get_payload_alt<P: DataProvider<HelloAltMarkerV1> + ?Sized>(
        provider: &P,
    ) -> Result<DataPayload<HelloAltMarkerV1>, DataError> {
        provider.load(Default::default()).map(|r| r.payload)
    }

    #[test]
    fn test_warehouse_owned() {
        let warehouse = get_warehouse(DATA);
        let hello_data = get_payload_v1(&warehouse).unwrap();
        assert!(matches!(
            hello_data.get(),
            HelloWorld {
                message: Cow::Borrowed(_),
            }
        ));
    }

    #[test]
    fn test_warehouse_owned_dyn_generic() {
        let warehouse = get_warehouse(DATA);
        let hello_data = get_payload_v1(&warehouse as &dyn DataProvider<HelloWorldV1>).unwrap();
        assert!(matches!(
            hello_data.get(),
            HelloWorld {
                message: Cow::Borrowed(_),
            }
        ));
    }

    #[test]
    fn test_provider2() {
        let warehouse = get_warehouse(DATA);
        let provider = DataProvider2::from(warehouse);
        let hello_data = get_payload_v1(&provider).unwrap();
        assert!(matches!(
            hello_data.get(),
            HelloWorld {
                message: Cow::Borrowed(_),
            }
        ));
    }

    #[test]
    fn test_provider2_dyn_generic() {
        let warehouse = get_warehouse(DATA);
        let provider = DataProvider2::from(warehouse);
        let hello_data = get_payload_v1(&provider as &dyn DataProvider<HelloWorldV1>).unwrap();
        assert!(matches!(
            hello_data.get(),
            HelloWorld {
                message: Cow::Borrowed(_),
            }
        ));
    }

    #[test]
    fn test_provider2_dyn_generic_alt() {
        let warehouse = get_warehouse(DATA);
        let provider = DataProvider2::from(warehouse);
        let hello_data = get_payload_alt(&provider as &dyn DataProvider<HelloAltMarkerV1>).unwrap();
        assert!(matches!(hello_data.get(), HelloAlt { .. }));
    }

    fn check_v1_v2<P>(d: &P)
    where
        P: DataProvider<HelloWorldV1> + DataProvider<HelloAltMarkerV1> + ?Sized,
    {
        let v1: DataPayload<HelloWorldV1> = d.load(Default::default()).unwrap().payload;
        let v2: DataPayload<HelloAltMarkerV1> = d.load(Default::default()).unwrap().payload;
        if v1.get().message == v2.get().message {
            panic!()
        }
    }

    #[test]
    fn test_v1_v2_generic() {
        let warehouse = get_warehouse(DATA);
        let provider = DataProvider2::from(warehouse);
        check_v1_v2(&provider);
    }
}
