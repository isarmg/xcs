#[test]

fn bounds() {
    fn check_send<T: Send>() {}
    fn check_sync<T: Sync>() {}
    fn check_unpin<T: Unpin>() {}
    // This has to take a value, since the async fn's return type is unnameable.
    fn check_send_sync_val<T: Send + Sync>(_t: T) {}

    check_send::<RwLock<u32>>();
    check_sync::<RwLock<u32>>();
    check_unpin::<RwLock<u32>>();

    check_send::<RwLockReadGuard<'_, u32>>();
    check_sync::<RwLockReadGuard<'_, u32>>();
    check_unpin::<RwLockReadGuard<'_, u32>>();

    check_send::<OwnedRwLockReadGuard<u32, i32>>();
    check_sync::<OwnedRwLockReadGuard<u32, i32>>();
    check_unpin::<OwnedRwLockReadGuard<u32, i32>>();

    check_send::<RwLockWriteGuard<'_, u32>>();
    check_sync::<RwLockWriteGuard<'_, u32>>();
    check_unpin::<RwLockWriteGuard<'_, u32>>();

    check_send::<RwLockMappedWriteGuard<'_, u32>>();
    check_sync::<RwLockMappedWriteGuard<'_, u32>>();
    check_unpin::<RwLockMappedWriteGuard<'_, u32>>();

    check_send::<OwnedRwLockWriteGuard<u32>>();
    check_sync::<OwnedRwLockWriteGuard<u32>>();
    check_unpin::<OwnedRwLockWriteGuard<u32>>();

    check_send::<OwnedRwLockMappedWriteGuard<u32, i32>>();
    check_sync::<OwnedRwLockMappedWriteGuard<u32, i32>>();
    check_unpin::<OwnedRwLockMappedWriteGuard<u32, i32>>();

    let rwlock = Arc::new(RwLock::new(0));
    check_send_sync_val(rwlock.read());
    check_send_sync_val(Arc::clone(&rwlock).read_owned());
    check_send_sync_val(rwlock.write());
    check_send_sync_val(Arc::clone(&rwlock).write_owned());
}
