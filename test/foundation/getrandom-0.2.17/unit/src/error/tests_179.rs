#[cfg(test)]
mod tests {
    use super::Error;
    use core::mem::size_of;

    #[test]
    fn test_size() {
        assert_eq!(size_of::<Error>(), 4);
        assert_eq!(size_of::<Result<(), Error>>(), 4);
    }
}
