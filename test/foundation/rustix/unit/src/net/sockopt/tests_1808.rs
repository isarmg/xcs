#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sizes() {
        use c::c_int;

        // Backend code needs to cast these to `c_int` so make sure that cast
        // isn't lossy.
        static_assertions::assert_eq_size!(Timeout, c_int);
    }
}
