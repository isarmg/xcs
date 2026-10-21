#[test]
fn search_tests_have_unique_names() {
    let assert = |constname, tests: &[SearchTest]| {
        let mut seen = HashMap::new(); // map from test name to position
        for (i, test) in tests.iter().enumerate() {
            if !seen.contains_key(test.name) {
                seen.insert(test.name, i);
            } else {
                let last = seen[test.name];
                panic!(
                    "{} tests have duplicate names at positions {} and {}",
                    constname, last, i
                );
            }
        }
    };
    assert("BASICS", BASICS);
    assert("LEFTMOST", LEFTMOST);
    assert("LEFTMOST_FIRST", LEFTMOST_FIRST);
    assert("LEFTMOST_LONGEST", LEFTMOST_LONGEST);
    assert("REGRESSION", REGRESSION);
    assert("TEDDY", TEDDY);
}
