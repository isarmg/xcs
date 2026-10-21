#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_drop_after_start() {
        {
            let (tx, rx) = mpsc::sync_channel(2);
            let _cpu_pool = ThreadPoolBuilder::new()
                .pool_size(2)
                .after_start(move |_| tx.send(1).unwrap())
                .create()
                .unwrap();

            // After ThreadPoolBuilder is deconstructed, the tx should be dropped
            // so that we can use rx as an iterator.
            let count = rx.into_iter().count();
            assert_eq!(count, 2);
        }
        std::thread::sleep(std::time::Duration::from_millis(500)); // wait for background threads closed: https://github.com/rust-lang/miri/issues/1371
    }
}
