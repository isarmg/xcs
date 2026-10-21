#[test]
    fn join_set_test() {
        let rt = Builder::new_current_thread().build().unwrap();

        let mut set = JoinSet::new();
        set.spawn_on(futures::future::ready(()), rt.handle());

        rt.block_on(set.join_next()).unwrap().unwrap();
    }
