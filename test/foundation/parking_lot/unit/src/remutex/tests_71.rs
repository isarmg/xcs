#[cfg(test)]
mod tests {
    use crate::ReentrantMutex;
    use crate::ReentrantMutexGuard;
    use std::cell::RefCell;
    use std::sync::mpsc::channel;
    use std::sync::Arc;
    use std::thread;

    #[cfg(feature = "serialization")]
    use bincode::{deserialize, serialize};

    #[test]
    fn smoke() {
        let m = ReentrantMutex::new(2);
        {
            let a = m.lock();
            {
                let b = m.lock();
                {
                    let c = m.lock();
                    assert_eq!(*c, 2);
                }
                assert_eq!(*b, 2);
            }
            assert_eq!(*a, 2);
        }
    }

    #[test]
    fn is_mutex() {
        let m = Arc::new(ReentrantMutex::new(RefCell::new(0)));
        let m2 = m.clone();
        let lock = m.lock();
        let child = thread::spawn(move || {
            let lock = m2.lock();
            assert_eq!(*lock.borrow(), 4950);
        });
        for i in 0..100 {
            let lock = m.lock();
            *lock.borrow_mut() += i;
        }
        drop(lock);
        child.join().unwrap();
    }

    #[test]
    fn trylock_works() {
        let m = Arc::new(ReentrantMutex::new(()));
        let m2 = m.clone();
        let _lock = m.try_lock();
        let _lock2 = m.try_lock();
        thread::spawn(move || {
            let lock = m2.try_lock();
            assert!(lock.is_none());
        })
        .join()
        .unwrap();
        let _lock3 = m.try_lock();
    }

    #[test]
    fn test_reentrant_mutex_debug() {
        let mutex = ReentrantMutex::new(vec![0u8, 10]);

        assert_eq!(format!("{:?}", mutex), "ReentrantMutex { data: [0, 10] }");
    }

    #[test]
    fn test_reentrant_mutex_bump() {
        let mutex = Arc::new(ReentrantMutex::new(()));
        let mutex2 = mutex.clone();

        let mut guard = mutex.lock();

        let (tx, rx) = channel();

        thread::spawn(move || {
            let _guard = mutex2.lock();
            tx.send(()).unwrap();
        });

        // `bump()` repeatedly until the thread starts up and requests the lock
        while rx.try_recv().is_err() {
            ReentrantMutexGuard::bump(&mut guard);
        }
    }

    #[cfg(feature = "serialization")]
    #[test]
    fn test_serialization() {
        let contents: Vec<u8> = vec![0, 1, 2];
        let mutex = ReentrantMutex::new(contents.clone());

        let serialized = serialize(&mutex).unwrap();
        let deserialized: ReentrantMutex<Vec<u8>> = deserialize(&serialized).unwrap();

        assert_eq!(*(mutex.lock()), *(deserialized.lock()));
        assert_eq!(contents, *(deserialized.lock()));
    }
}
