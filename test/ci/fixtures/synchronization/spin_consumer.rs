#![no_std]
pub fn generic_lock() {
    let lock = spin::lock_api::Mutex::new(1);
    assert_eq!(*lock.lock(), 1);
}
