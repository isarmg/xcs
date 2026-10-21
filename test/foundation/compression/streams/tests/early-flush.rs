use std::io::{Read, Write};

use compression::streams::read::GzDecoder;
use compression::streams::write::GzEncoder;

#[test]
fn smoke() {
    let mut w = GzEncoder::new(Vec::new(), compression::streams::Compression::default());
    w.flush().unwrap();
    w.write_all(b"hello").unwrap();

    let bytes = w.finish().unwrap();

    let mut r = GzDecoder::new(&bytes[..]);
    let mut s = String::new();
    r.read_to_string(&mut s).unwrap();
    assert_eq!(s, "hello");
}
