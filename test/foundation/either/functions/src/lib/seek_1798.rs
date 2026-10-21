#[test]
fn seek() {
    use std::io;

    let use_empty = false;
    let mut mockdata = [0x00; 256];
    for (i, data) in mockdata.iter_mut().enumerate() {
        *data = i as u8;
    }

    let mut reader = if use_empty {
        // Empty didn't impl Seek until Rust 1.51
        Left(io::Cursor::new([]))
    } else {
        Right(io::Cursor::new(&mockdata[..]))
    };

    let mut buf = [0u8; 16];
    assert_eq!(reader.read(&mut buf).unwrap(), buf.len());
    assert_eq!(buf, mockdata[..buf.len()]);

    // the first read should advance the cursor and return the next 16 bytes thus the `ne`
    assert_eq!(reader.read(&mut buf).unwrap(), buf.len());
    assert_ne!(buf, mockdata[..buf.len()]);

    // if the seek operation fails it should read 16..31 instead of 0..15
    reader.seek(io::SeekFrom::Start(0)).unwrap();
    assert_eq!(reader.read(&mut buf).unwrap(), buf.len());
    assert_eq!(buf, mockdata[..buf.len()]);
}
