/// Produces fewer items than promised. Does not do any
/// splits at all.
#[test]
#[should_panic(expected = "expected 5 total writes, but got 2")]
fn produce_fewer_items() {
    let mut v = vec![];
    collect_with_consumer(&mut v, 5, |consumer| {
        let mut folder = consumer.into_folder();
        folder = folder.consume(22);
        folder = folder.consume(23);
        folder.complete()
    });
}
