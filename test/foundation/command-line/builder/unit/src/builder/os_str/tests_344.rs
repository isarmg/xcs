#[cfg(test)]
#[cfg(feature = "string")]
mod tests {
    use super::*;

    #[test]
    #[cfg(feature = "string")]
    fn from_cow_borrowed() {
        let cow = Cow::Borrowed("hello");
        let osstr = OsStr::from(cow);
        assert_eq!(osstr, OsStr::from("hello"));
    }

    #[test]
    #[cfg(feature = "string")]
    fn from_cow_owned() {
        let cow = Cow::Owned("world".to_owned());
        let osstr = OsStr::from(cow);
        assert_eq!(osstr, OsStr::from("world"));
    }
}
