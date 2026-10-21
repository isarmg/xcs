use compression::codec::{
    deflate::compress_to_vec_zlib,
    inflate::{
        core::{decompress, inflate_flags, DecompressorOxide},
        TINFLStatus,
    },
};

#[test]
fn serialized_decoder_checkpoint_resumes_identically() {
    let data: Vec<u8> = (0..16384).map(|i| (i % 251) as u8).collect();
    let encoded = compress_to_vec_zlib(&data, 6);
    let mut original = DecompressorOxide::new();
    let mut buffer = vec![0; data.len()];
    let flags = inflate_flags::TINFL_FLAG_PARSE_ZLIB_HEADER
        | inflate_flags::TINFL_FLAG_USING_NON_WRAPPING_OUTPUT_BUF;
    let (status, consumed, produced) = decompress(
        &mut original,
        &encoded[..encoded.len() / 2],
        &mut buffer,
        0,
        flags | inflate_flags::TINFL_FLAG_HAS_MORE_INPUT,
    );
    assert_eq!(status, TINFLStatus::NeedsMoreInput);
    let snapshot = json_codec::to_vec(&original).unwrap();
    let mut restored: DecompressorOxide = json_codec::from_slice(&snapshot).unwrap();
    assert_eq!(json_codec::to_vec(&restored).unwrap(), snapshot);
    let mut resumed_buffer = buffer.clone();
    let expected = decompress(
        &mut original,
        &encoded[consumed..],
        &mut buffer,
        produced,
        flags,
    );
    let actual = decompress(
        &mut restored,
        &encoded[consumed..],
        &mut resumed_buffer,
        produced,
        flags,
    );
    assert_eq!(actual, expected);
    assert_eq!(actual.0, TINFLStatus::Done);
    assert_eq!(resumed_buffer, data);
    assert_eq!(resumed_buffer, buffer);
}
