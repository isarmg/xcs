#[cfg(test)]
mod tests {
    use crate::error;
    use crate::testutil as test;

    #[test]
    fn one_ok() {
        test::run(test_vector_file!("../../fixtures/src/test_1_tests.txt"), |_, test_case| {
            let _ = test_case.consume_string("Key");
            Ok(())
        });
    }

    #[test]
    #[should_panic(expected = "Test failed.")]
    fn one_err() {
        test::run(test_vector_file!("../../fixtures/src/test_1_tests.txt"), |_, test_case| {
            let _ = test_case.consume_string("Key");
            Err(error::Unspecified)
        });
    }

    #[test]
    #[should_panic(expected = "Oh noes!")]
    fn one_panics() {
        test::run(test_vector_file!("../../fixtures/src/test_1_tests.txt"), |_, test_case| {
            let _ = test_case.consume_string("Key");
            panic!("Oh noes!");
        });
    }

    #[test]
    #[should_panic(expected = "Test failed.")]
    fn first_err() {
        err_one(0)
    }

    #[test]
    #[should_panic(expected = "Test failed.")]
    fn middle_err() {
        err_one(1)
    }

    #[test]
    #[should_panic(expected = "Test failed.")]
    fn last_err() {
        err_one(2)
    }

    fn err_one(test_to_fail: usize) {
        let mut n = 0;
        test::run(test_vector_file!("../../fixtures/src/test_3_tests.txt"), |_, test_case| {
            let _ = test_case.consume_string("Key");
            let result = if n != test_to_fail {
                Ok(())
            } else {
                Err(error::Unspecified)
            };
            n += 1;
            result
        });
    }

    #[test]
    #[should_panic(expected = "Oh Noes!")]
    fn first_panic() {
        panic_one(0)
    }

    #[test]
    #[should_panic(expected = "Oh Noes!")]
    fn middle_panic() {
        panic_one(1)
    }

    #[test]
    #[should_panic(expected = "Oh Noes!")]
    fn last_panic() {
        panic_one(2)
    }

    fn panic_one(test_to_fail: usize) {
        let mut n = 0;
        test::run(test_vector_file!("../../fixtures/src/test_3_tests.txt"), |_, test_case| {
            let _ = test_case.consume_string("Key");
            if n == test_to_fail {
                panic!("Oh Noes!");
            };
            n += 1;
            Ok(())
        });
    }

    #[test]
    #[should_panic(expected = "Syntax error: Expected Key = Value.")]
    fn syntax_error() {
        test::run(
            test_vector_file!("../../fixtures/src/test_1_syntax_error_tests.txt"),
            |_, _| Ok(()),
        );
    }
}
