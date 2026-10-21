#[test]
#[cfg_attr(not(panic = "unwind"), ignore)]
fn left_produces_fewer_items_drops() {
    let counter = DropCounter::default();
    let mut v = vec![];
    let panic_result = panic::catch_unwind(panic::AssertUnwindSafe(|| {
        collect_with_consumer(&mut v, 4, |consumer| {
            let reducer = consumer.to_reducer();
            let (left_consumer, right_consumer, _) = consumer.split_at(2);
            let mut left_folder = left_consumer.into_folder();
            let mut right_folder = right_consumer.into_folder();
            left_folder = left_folder.consume(counter.element());
            right_folder = right_folder
                .consume(counter.element())
                .consume(counter.element());
            let left_result = left_folder.complete();
            let right_result = right_folder.complete();
            reducer.reduce(left_result, right_result)
        });
    }));
    assert!(v.is_empty());
    assert_is_panic_with_message(&panic_result, "expected 4 total writes, but got 1");
    counter.assert_drop_count();
}
