#[test]
#[should_panic(expected = "expected 4 total writes, but got 1")]
fn left_produces_fewer_items() {
    let mut v = vec![];
    collect_with_consumer(&mut v, 4, |consumer| {
        let reducer = consumer.to_reducer();
        let (left_consumer, right_consumer, _) = consumer.split_at(2);
        let mut left_folder = left_consumer.into_folder();
        let mut right_folder = right_consumer.into_folder();
        left_folder = left_folder.consume(0);
        right_folder = right_folder.consume(2).consume(3);
        let left_result = left_folder.complete();
        let right_result = right_folder.complete();
        reducer.reduce(left_result, right_result)
    });
}
