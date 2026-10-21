#[test]
fn test_debug() {
    use crate::hello_world::*;
    use crate::prelude::*;
    let resp = HelloWorldProvider
        .load(DataRequest {
            id: DataIdentifierBorrowed::for_locale(&icu_locale_core::data_locale!("en")),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(
        "DataResponse { metadata: DataResponseMetadata { locale: None, buffer_format: None, checksum: Some(1234) }, payload: HelloWorld { message: \"Hello World\" } }",
        format!("{resp:?}")
    );
}
