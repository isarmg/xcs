#[cfg(test)]
#[cfg(feature = "string")]
mod tests {
    use super::*;

    #[test]
    #[cfg(feature = "string")]
    fn from_cow_borrowed() {
        let cow = Cow::Borrowed("hello");
        let str = Str::from(cow);
        assert_eq!(str, Str::from("hello"));
    }

    #[test]
    #[cfg(feature = "string")]
    fn from_cow_owned() {
        let cow = Cow::Owned("world".to_owned());
        let str = Str::from(cow);
        assert_eq!(str, Str::from("world"));
    }
}
