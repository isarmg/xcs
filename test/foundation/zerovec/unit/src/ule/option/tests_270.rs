#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(debug_assertions)]
    #[should_panic]
    fn test_option_varule_underflow() {
        unsafe {
            let _ = OptionVarULE::<str>::from_bytes_unchecked(&[]);
        }
    }
}
