#[test]
#[should_panic(expected = "left consumer panic")]
fn left_panics() {
    let mut v = vec![];
    collect_with_consumer(&mut v, 4, |consumer| {
        let reducer = consumer.to_reducer();
        let (left_consumer, right_consumer, _) = consumer.split_at(2);
        let (left_result, right_result) = join(
            || {
                let mut left_folder = left_consumer.into_folder();
                left_folder = left_folder.consume(0);
                panic!("left consumer panic");
            },
            || {
                let mut right_folder = right_consumer.into_folder();
                right_folder = right_folder.consume(2);
                right_folder.complete() // early return
            },
        );
        reducer.reduce(left_result, right_result)
    });
    unreachable!();
}
