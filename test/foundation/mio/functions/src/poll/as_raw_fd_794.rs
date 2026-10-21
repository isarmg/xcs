#[cfg(all(
        unix,
        not(mio_unsupported_force_poll_poll),
        not(any(
            target_os = "aix",
            target_os = "espidf",
            target_os = "nuttx",
            target_os = "hermit",
            target_os = "hurd",
            target_os = "nto",
            target_os = "vita",
            target_os = "cygwin",
            target_os = "horizon"
        )),
    ))]
    #[test]
    pub fn as_raw_fd() {
        let poll = Poll::new().unwrap();
        assert!(poll.as_raw_fd() > 0);
    }
