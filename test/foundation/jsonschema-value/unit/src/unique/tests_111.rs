#[cfg(test)]
mod tests {
    use super::{is_unique, ITEMS_SIZE_THRESHOLD};
    use serde_json::{json, Value};
    use test_case::test_case;

    #[test_case(&[] => true; "empty array")]
    #[test_case(&[json!(1)] => true; "one element array")]
    #[test_case(&[json!(1), json!(2)] => true; "two unique elements")]
    #[test_case(&[json!(1), json!(1)] => false; "two non-unique elements")]
    #[test_case(&[json!(1), json!(2), json!(3)] => true; "three unique elements")]
    #[test_case(&[json!(1), json!(2), json!(1)] => false; "three non-unique elements")]
    #[test_case(&[json!(1), json!(2), json!(3), json!(4), json!(5), json!(6), json!(7), json!(8), json!(9), json!(10), json!(11), json!(12), json!(13), json!(14), json!(15), json!(1.0)] => false; "positive numbers with fractions")]
    #[test_case(&[json!(-1), json!(-2), json!(-3), json!(-4), json!(-5), json!(-6), json!(-7), json!(-8), json!(-9), json!(-10), json!(-11), json!(-12), json!(-13), json!(-14), json!(-15), json!(-1.0)] => false; "negative numbers with fractions")]
    #[test_case(&[json!(1), json!("string"), json!(true), json!(null), json!({"key": "value"}), json!([1, 2, 3])] => true; "mixed types")]
    #[test_case(&[json!({"a": 1, "b": 1}), json!({"a": 1, "b": 2}), json!({"a": 1, "b": 3})] => true; "complex objects unique")]
    #[test_case(&[json!({"a": 1, "b": 2}), json!({"b": 2, "a": 1}), json!({"a": 1, "b": 2})] => false; "complex objects non-unique")]
    #[test_case(&[json!(0), json!(1), json!(2), json!(-0.0)] => false; "zero and negative zero")]
    #[test_case(&[json!(0), json!(1), json!(2), json!(3), json!(4), json!(5), json!(6), json!(7), json!(8), json!(9), json!(10), json!(11), json!(12), json!(13), json!(14), json!(15), json!(-0.0)] => false; "zero and negative zero beyond the pairwise threshold")]
    #[test_case(&[json!({"a": 0}), json!({"a": 1}), json!({"a": 2}), json!({"a": 3}), json!({"a": 4}), json!({"a": 5}), json!({"a": 6}), json!({"a": 7}), json!({"a": 8}), json!({"a": 9}), json!({"a": 10}), json!({"a": 11}), json!({"a": 12}), json!({"a": 13}), json!({"a": 14}), json!({"a": 15}), json!({"a": -0.0})] => false; "zero and negative zero inside objects beyond the pairwise threshold")]
    fn test_is_unique(items: &[Value]) -> bool {
        is_unique(items)
    }

    #[test_case(ITEMS_SIZE_THRESHOLD => true; "small array unique")]
    #[test_case(ITEMS_SIZE_THRESHOLD + 1 => true; "large array unique")]
    fn test_unique_arrays(size: usize) -> bool {
        let arr = (1..=size).map(|i| json!(i)).collect::<Vec<_>>();
        is_unique(&arr)
    }

    #[test_case(ITEMS_SIZE_THRESHOLD => false; "small array non-unique")]
    #[test_case(ITEMS_SIZE_THRESHOLD + 1 => false; "large array non-unique")]
    fn test_non_unique_arrays(size: usize) -> bool {
        let mut arr = (1..=size).map(|i| json!(i)).collect::<Vec<_>>();
        arr[size - 1] = json!(1);
        is_unique(&arr)
    }
}
