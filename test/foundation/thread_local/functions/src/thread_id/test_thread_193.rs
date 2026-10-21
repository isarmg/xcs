#[test]
fn test_thread() {
    let thread = Thread::new(0);
    assert_eq!(thread.id(), 0);
    assert_eq!(thread.bucket, 0);
    assert_eq!(thread.bucket_size(), 1);
    assert_eq!(thread.index, 0);

    let thread = Thread::new(1);
    assert_eq!(thread.id(), 1);
    assert_eq!(thread.bucket, 1);
    assert_eq!(thread.bucket_size(), 2);
    assert_eq!(thread.index, 0);

    let thread = Thread::new(2);
    assert_eq!(thread.id(), 2);
    assert_eq!(thread.bucket, 1);
    assert_eq!(thread.bucket_size(), 2);
    assert_eq!(thread.index, 1);

    let thread = Thread::new(3);
    assert_eq!(thread.id(), 3);
    assert_eq!(thread.bucket, 2);
    assert_eq!(thread.bucket_size(), 4);
    assert_eq!(thread.index, 0);

    let thread = Thread::new(19);
    assert_eq!(thread.id(), 19);
    assert_eq!(thread.bucket, 4);
    assert_eq!(thread.bucket_size(), 16);
    assert_eq!(thread.index, 4);
}
