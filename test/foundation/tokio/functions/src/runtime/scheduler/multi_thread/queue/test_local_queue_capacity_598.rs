#[test]
fn test_local_queue_capacity() {
    assert!(LOCAL_QUEUE_CAPACITY - 1 <= u8::MAX as usize);
}
