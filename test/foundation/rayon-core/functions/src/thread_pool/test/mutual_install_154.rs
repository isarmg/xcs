#[test]
#[cfg_attr(any(target_os = "emscripten", target_family = "wasm"), ignore)]
fn mutual_install() {
    let pool1 = ThreadPoolBuilder::new().num_threads(1).build().unwrap();
    let pool2 = ThreadPoolBuilder::new().num_threads(1).build().unwrap();

    let ok = pool1.install(|| {
        // This creates a dependency from `pool1` -> `pool2`
        pool2.install(|| {
            // This creates a dependency from `pool2` -> `pool1`
            pool1.install(|| {
                // If they blocked on inter-pool installs, there would be no
                // threads left to run this!
                true
            })
        })
    });
    assert!(ok);
}
