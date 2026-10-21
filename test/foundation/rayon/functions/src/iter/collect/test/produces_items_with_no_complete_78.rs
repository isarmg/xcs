#[test]
#[cfg_attr(not(panic = "unwind"), ignore)]
fn produces_items_with_no_complete() {
    let counter = DropCounter::default();
    let mut v = vec![];
    let panic_result = panic::catch_unwind(panic::AssertUnwindSafe(|| {
        collect_with_consumer(&mut v, 2, |consumer| {
            let mut folder = consumer.into_folder();
            folder = folder.consume(counter.element());
            folder = folder.consume(counter.element());
            panic!("folder does not complete");
        });
    }));
    assert!(v.is_empty());
    assert_is_panic_with_message(&panic_result, "folder does not complete");
    counter.assert_drop_count();
}
