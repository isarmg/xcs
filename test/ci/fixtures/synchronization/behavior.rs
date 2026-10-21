use std::sync::{Arc, Barrier};
use synchronization::{Condvar, Mutex, MutexGuard, ReentrantMutex, RwLock, RwLockWriteGuard};
#[test]
fn contention_and_fair_handoff() {
    let value = Arc::new(Mutex::new(0));
    let start = Arc::new(Barrier::new(5));
    let threads: Vec<_> = (0..4)
        .map(|_| {
            let value = value.clone();
            let start = start.clone();
            std::thread::spawn(move || {
                start.wait();
                for _ in 0..1000 {
                    let mut g = value.lock();
                    *g += 1;
                    MutexGuard::unlock_fair(g);
                }
            })
        })
        .collect();
    start.wait();
    for t in threads {
        t.join().unwrap();
    }
    assert_eq!(*value.lock(), 4000);
}
#[test]
fn mapped_and_owned_guards() {
    let value = Mutex::new((1, 2));
    let mut mapped = MutexGuard::map(value.lock(), |v| &mut v.1);
    *mapped = 9;
    drop(mapped);
    assert_eq!(*value.lock(), (1, 9));
    #[cfg(feature = "arc")]
    {
        let value = Arc::new(Mutex::new(3));
        let mut guard = value.lock_arc();
        drop(value);
        *guard += 1;
        assert_eq!(*guard, 4);
    }
}
#[test]
fn recursive_and_upgrade_contracts() {
    let value = ReentrantMutex::new(std::cell::Cell::new(1));
    let a = value.lock();
    let b = value.lock();
    b.set(4);
    assert_eq!(a.get(), 4);
    drop((a, b));
    let value = RwLock::new(1);
    let mut w = value.write();
    *w = 3;
    let read = RwLockWriteGuard::downgrade(w);
    assert_eq!(*read, 3);
    assert!(value.try_write().is_none());
    drop(read);
    assert!(value.try_write().is_some());
}
#[test]
fn predicate_wakeup_and_timeout() {
    let pair = Arc::new((Mutex::new(false), Condvar::new()));
    let worker = pair.clone();
    let t = std::thread::spawn(move || {
        let mut ready = worker.0.lock();
        *ready = true;
        worker.1.notify_one();
    });
    let mut ready = pair.0.lock();
    pair.1.wait_while(&mut ready, |v| !(*v));
    assert!(*ready);
    drop(ready);
    t.join().unwrap();
    let guard = pair.0.lock();
    assert!(pair
        .0
        .try_lock_for(std::time::Duration::from_millis(1))
        .is_none());
    drop(guard);
}
#[test]
fn no_poison_after_panic() {
    let mutex = Arc::new(Mutex::new(1));
    let other = mutex.clone();
    assert!(std::thread::spawn(move || {
        let mut guard = other.lock();
        *guard = 7;
        panic!("intentional");
    })
    .join()
    .is_err());
    assert_eq!(*mutex.lock(), 7);
}
#[cfg(feature = "serialization")]
#[test]
fn serialization_round_trip() {
    let mutex = Mutex::new(123u32);
    let text = json_codec::to_string(&mutex).unwrap();
    assert_eq!(text, "123");
    let restored: Mutex<u32> = json_codec::from_str(&text).unwrap();
    assert_eq!(*restored.lock(), 123);
}
