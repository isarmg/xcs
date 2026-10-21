#[test]

fn bounds() {
    fn check_send<T: Send>() {}
    fn check_unpin<T: Unpin>() {}
    // This has to take a value, since the async fn's return type is unnameable.
    fn check_send_sync_val<T: Send + Sync>(_t: T) {}
    fn check_send_sync<T: Send + Sync>() {}
    fn check_static<T: 'static>() {}
    fn check_static_val<T: 'static>(_t: T) {}

    check_send::<MutexGuard<'_, u32>>();
    check_send::<OwnedMutexGuard<u32>>();
    check_unpin::<Mutex<u32>>();
    check_send_sync::<Mutex<u32>>();
    check_static::<OwnedMutexGuard<u32>>();

    let mutex = Mutex::new(1);
    check_send_sync_val(mutex.lock());
    let arc_mutex = Arc::new(Mutex::new(1));
    check_send_sync_val(arc_mutex.clone().lock_owned());
    check_static_val(arc_mutex.lock_owned());
}
