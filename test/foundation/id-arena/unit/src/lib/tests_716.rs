#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        struct Foo;
        assert_send_sync::<Id<Foo>>();
    }
}
