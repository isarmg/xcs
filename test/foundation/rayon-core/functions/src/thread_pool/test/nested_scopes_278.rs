#[test]
#[cfg_attr(any(target_os = "emscripten", target_family = "wasm"), ignore)]
fn nested_scopes() {
    // Create matching scopes for every thread pool.
    fn nest<'scope, OP>(pools: &[ThreadPool], scopes: Vec<&Scope<'scope>>, op: OP)
    where
        OP: FnOnce(&[&Scope<'scope>]) + Send,
    {
        if let Some((pool, tail)) = pools.split_first() {
            pool.scope(move |s| {
                // This move reduces the reference lifetimes by variance to match s,
                // but the actual scopes are still tied to the invariant 'scope.
                let mut scopes = scopes;
                scopes.push(s);
                nest(tail, scopes, op)
            })
        } else {
            (op)(&scopes)
        }
    }

    let pools: Vec<_> = (0..10)
        .map(|_| ThreadPoolBuilder::new().num_threads(1).build().unwrap())
        .collect();

    let counter = AtomicUsize::new(0);
    nest(&pools, vec![], |scopes| {
        for &s in scopes {
            s.spawn(|_| {
                // Our 'scope lets us borrow the counter in every pool.
                counter.fetch_add(1, Ordering::Relaxed);
            });
        }
    });
    assert_eq!(counter.into_inner(), pools.len());
}
