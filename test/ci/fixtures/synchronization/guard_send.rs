fn assert_send<T: Send>(_: T) {}
pub fn guard_must_be_send() {
    let mutex = synchronization::Mutex::new(1);
    assert_send(mutex.lock());
}
