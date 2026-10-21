#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    fn parse_kebab_name(s: &str) -> Option<ComponentName> {
        ComponentName::new(s, 0).ok()
    }

    #[test]
    fn kebab_smoke() {
        assert!(KebabStr::new("").is_none());
        assert!(KebabStr::new("a").is_some());
        assert!(KebabStr::new("aB").is_none());
        assert!(KebabStr::new("a-B").is_some());
        assert!(KebabStr::new("a-").is_none());
        assert!(KebabStr::new("-").is_none());
        assert!(KebabStr::new("¶").is_none());
        assert!(KebabStr::new("0").is_none());
        assert!(KebabStr::new("a0").is_some());
        assert!(KebabStr::new("a-0").is_some());
        assert!(KebabStr::new("0-a").is_none());
        assert!(KebabStr::new("a-b--c").is_none());
        assert!(KebabStr::new("a0-000-3d4a-54FF").is_some());
        assert!(KebabStr::new("a0-000-3d4A-54Ff").is_none());
    }

    #[test]
    fn name_smoke() {
        assert!(parse_kebab_name("a").is_some());
        assert!(parse_kebab_name("[foo]a").is_none());
        assert!(parse_kebab_name("[constructor]a").is_some());
        assert!(parse_kebab_name("[method]a").is_none());
        assert!(parse_kebab_name("[method]a.b").is_some());
        assert!(parse_kebab_name("[method]a-0.b-1").is_some());
        assert!(parse_kebab_name("[method]a.b.c").is_none());
        assert!(parse_kebab_name("[static]a.b").is_some());
        assert!(parse_kebab_name("[static]a").is_none());
    }

    #[test]
    fn name_equality() {
        assert_eq!(parse_kebab_name("a"), parse_kebab_name("a"));
        assert_ne!(parse_kebab_name("a"), parse_kebab_name("b"));
        assert_eq!(
            parse_kebab_name("[constructor]a"),
            parse_kebab_name("[constructor]a")
        );
        assert_ne!(
            parse_kebab_name("[constructor]a"),
            parse_kebab_name("[constructor]b")
        );
        assert_eq!(
            parse_kebab_name("[method]a.b"),
            parse_kebab_name("[method]a.b")
        );
        assert_ne!(
            parse_kebab_name("[method]a.b"),
            parse_kebab_name("[method]b.b")
        );
        assert_eq!(
            parse_kebab_name("[static]a.b"),
            parse_kebab_name("[static]a.b")
        );
        assert_ne!(
            parse_kebab_name("[static]a.b"),
            parse_kebab_name("[static]b.b")
        );

        assert_eq!(
            parse_kebab_name("[static]a.b"),
            parse_kebab_name("[method]a.b")
        );
        assert_eq!(
            parse_kebab_name("[method]a.b"),
            parse_kebab_name("[static]a.b")
        );

        assert_ne!(
            parse_kebab_name("[method]b.b"),
            parse_kebab_name("[static]a.b")
        );

        let mut s = HashSet::new();
        assert!(s.insert(parse_kebab_name("a")));
        assert!(s.insert(parse_kebab_name("[constructor]a")));
        assert!(s.insert(parse_kebab_name("[method]a.b")));
        assert!(!s.insert(parse_kebab_name("[static]a.b")));
        assert!(s.insert(parse_kebab_name("[static]b.b")));
    }
}
