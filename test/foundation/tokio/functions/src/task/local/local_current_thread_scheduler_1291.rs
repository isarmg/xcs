#[test]
    fn local_current_thread_scheduler() {
        let f = async {
            LocalSet::new()
                .run_until(async {
                    spawn_local(async {}).await.unwrap();
                })
                .await;
        };
        crate::runtime::Builder::new_current_thread()
            .build()
            .expect("rt")
            .block_on(f)
    }
