#[test]
fn test_needs_drop() {
    use core::mem::needs_drop;

    struct NeedsDrop;

    impl Drop for NeedsDrop {
        fn drop(&mut self) {}
    }

    assert!(needs_drop::<NeedsDrop>());

    // Test each of the types with a handwritten TrivialDrop impl above.
    assert!(!needs_drop::<iter::Empty<NeedsDrop>>());
    assert!(!needs_drop::<slice::Iter<NeedsDrop>>());
    assert!(!needs_drop::<slice::IterMut<NeedsDrop>>());
    assert!(!needs_drop::<option::IntoIter<&NeedsDrop>>());
    assert!(!needs_drop::<option::IntoIter<&mut NeedsDrop>>());
}
