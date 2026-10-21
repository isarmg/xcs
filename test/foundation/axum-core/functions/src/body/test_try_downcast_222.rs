#[test]
fn test_try_downcast() {
    assert_eq!(try_downcast::<i32, _>(5_u32), Err(5_u32));
    assert_eq!(try_downcast::<i32, _>(5_i32), Ok(5_i32));
}
