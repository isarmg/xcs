#[cfg(test)]
mod tests {
    use super::*;

    /// A simple type that does not implement [`std::fmt::Debug`].
    struct NotDebug;

    fn is_debug<T: std::fmt::Debug>() {}

    #[test]
    fn assert_debug() {
        is_debug::<JoinQueue<NotDebug>>();
    }
}
