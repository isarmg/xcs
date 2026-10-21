#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_from_string() {
        assert_eq!("a key", Key::from_str("a key").as_str());
    }

    #[test]
    fn key_to_borrowed() {
        assert_eq!("a key", Key::from_str("a key").to_borrowed_str().unwrap());
    }

    #[test]
    fn key_static_eq() {
        assert_eq!(Key::from_str_static("a key"), Key::from_str_static("a key"));
    }

    #[test]
    fn key_borrowed_eq() {
        const KEY: &str = "a key";
        let k1 = String::from(KEY);
        let k2 = String::from(KEY);
        assert_eq!(Key::from_str(&k1), Key::from_str(&k2));
    }

    #[test]
    fn key_borrowed_and_static_eq() {
        const KEY: &str = "a key";
        let static_key = Key::from_str_static(KEY);
        let borrowed_key = Key::from_str(KEY);
        assert_eq!(static_key, borrowed_key);
    }
}
