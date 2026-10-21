#[cfg(test)]
mod tests {
    use super::Connected;

    #[derive(Clone, Debug, PartialEq)]
    struct Ex1(usize);

    #[derive(Clone, Debug, PartialEq)]
    struct Ex2(&'static str);

    #[derive(Clone, Debug, PartialEq)]
    struct Ex3(&'static str);

    #[test]
    fn test_connected_extra() {
        let c1 = Connected::new().extra(Ex1(41));

        let mut ex = ::http::Extensions::new();

        assert_eq!(ex.get::<Ex1>(), None);

        c1.extra.as_ref().expect("c1 extra").set(&mut ex);

        assert_eq!(ex.get::<Ex1>(), Some(&Ex1(41)));
    }

    #[test]
    fn test_connected_extra_chain() {
        // If a user composes connectors and at each stage, there's "extra"
        // info to attach, it shouldn't override the previous extras.

        let c1 = Connected::new()
            .extra(Ex1(45))
            .extra(Ex2("zoom"))
            .extra(Ex3("pew pew"));

        let mut ex1 = ::http::Extensions::new();

        assert_eq!(ex1.get::<Ex1>(), None);
        assert_eq!(ex1.get::<Ex2>(), None);
        assert_eq!(ex1.get::<Ex3>(), None);

        c1.extra.as_ref().expect("c1 extra").set(&mut ex1);

        assert_eq!(ex1.get::<Ex1>(), Some(&Ex1(45)));
        assert_eq!(ex1.get::<Ex2>(), Some(&Ex2("zoom")));
        assert_eq!(ex1.get::<Ex3>(), Some(&Ex3("pew pew")));

        // Just like extensions, inserting the same type overrides previous type.
        let c2 = Connected::new()
            .extra(Ex1(33))
            .extra(Ex2("hiccup"))
            .extra(Ex1(99));

        let mut ex2 = ::http::Extensions::new();

        c2.extra.as_ref().expect("c2 extra").set(&mut ex2);

        assert_eq!(ex2.get::<Ex1>(), Some(&Ex1(99)));
        assert_eq!(ex2.get::<Ex2>(), Some(&Ex2("hiccup")));
    }
}
