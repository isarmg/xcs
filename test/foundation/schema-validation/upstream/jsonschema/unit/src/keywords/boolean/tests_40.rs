#[cfg(test)]
mod tests {
    use crate::tests_util;
    use json_codec::json;

    #[test]
    fn location() {
        tests_util::assert_schema_location(&json!(false), &json!(1), "");
    }
}
