#[cfg(test)]
mod tests {
    #[test]
    fn test_macro_construction() {
        let s1 = tinystr!(8, "foobar");
        assert_eq!(&*s1, "foobar");

        let s1 = tinystr!(12, "foobarbaz");
        assert_eq!(&*s1, "foobarbaz");
    }
}
