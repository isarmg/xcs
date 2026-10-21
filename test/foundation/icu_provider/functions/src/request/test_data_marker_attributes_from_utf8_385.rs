#[test]
fn test_data_marker_attributes_from_utf8() {
    let bytes_vec: Vec<&[u8]> = vec![
        b"long-meter",
        b"long",
        b"meter",
        b"short-meter-second",
        b"usd",
    ];

    for bytes in bytes_vec {
        let marker = DataMarkerAttributes::try_from_utf8(bytes).unwrap();
        assert_eq!(marker.to_string().as_bytes(), bytes);
    }
}
