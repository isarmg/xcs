/// Test that custom spawned threads get their `WorkerThread` cleared once
/// the pool is done with them, allowing them to be used with rayon again
/// later. e.g. WebAssembly want to have their own pool of available threads.
#[test]
#[cfg_attr(any(target_os = "emscripten", target_family = "wasm"), ignore)]
fn cleared_current_thread() -> Result<(), ThreadPoolBuildError> {
    let n_threads = 5;
    let mut handles = vec![];
    let pool = ThreadPoolBuilder::new()
        .num_threads(n_threads)
        .spawn_handler(|thread| {
            let handle = std::thread::spawn(move || {
                thread.run();

                // Afterward, the current thread shouldn't be set anymore.
                assert_eq!(crate::current_thread_index(), None);
            });
            handles.push(handle);
            Ok(())
        })
        .build()?;
    assert_eq!(handles.len(), n_threads);

    pool.install(|| assert!(crate::current_thread_index().is_some()));
    drop(pool);

    // Wait for all threads to make their assertions and exit
    for handle in handles {
        handle.join().unwrap();
    }

    Ok(())
}
