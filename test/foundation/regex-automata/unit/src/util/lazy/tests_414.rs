#[cfg(test)]
mod tests {
    use super::*;

    fn assert_send<T: Send>() {}
    fn assert_sync<T: Sync>() {}
    fn assert_unwind<T: core::panic::UnwindSafe>() {}
    fn assert_refunwind<T: core::panic::RefUnwindSafe>() {}

    #[test]
    fn oibits() {
        assert_send::<Lazy<u64>>();
        assert_sync::<Lazy<u64>>();
        assert_unwind::<Lazy<u64>>();
        assert_refunwind::<Lazy<u64>>();
    }

    // This is a regression test because we used to rely on the inferred Sync
    // impl for the Lazy type defined above (for 'alloc' mode). In the
    // inferred impl, it only requires that T: Sync for Lazy<T>: Sync. But
    // if we have that, we can actually make use of the fact that Lazy<T> drops
    // T to create a value on one thread and drop it on another. This *should*
    // require T: Send, but our missing bounds before let it sneak by.
    //
    // Basically, this test should not compile, so we... comment it out. We
    // don't have a great way of testing compile-fail tests right now.
    //
    // See: https://github.com/BurntSushi/regex-automata/issues/30
    /*
    #[test]
    fn sync_not_send() {
        #[allow(dead_code)]
        fn inner<T: Sync + Default>() {
            let lazy = Lazy::new(move || T::default());
            std::thread::scope(|scope| {
                scope.spawn(|| {
                    Lazy::get(&lazy); // We create T in this thread
                });
            });
            // And drop in this thread.
            drop(lazy);
            // So we have send a !Send type over threads. (with some more
            // legwork, its possible to even sneak the value out of drop
            // through thread local)
        }
    }
    */
}
