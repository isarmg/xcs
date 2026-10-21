#[cfg(test)]
#[cfg(feature = "deadlock_detection")]
mod tests {
    use crate::locks::{Mutex, ReentrantMutex, RwLock};
    use std::sync::{Arc, Barrier};
    use std::thread::{self, sleep};
    use std::time::Duration;

    // We need to serialize these tests since deadlock detection uses global state
    static DEADLOCK_DETECTION_LOCK: Mutex<()> = crate::locks::const_mutex(());

    fn check_deadlock() -> bool {
        use crate::parking::deadlock::check_deadlock;
        !check_deadlock().is_empty()
    }

    #[test]
    fn test_mutex_deadlock() {
        let _guard = DEADLOCK_DETECTION_LOCK.lock();

        let m1: Arc<Mutex<()>> = Default::default();
        let m2: Arc<Mutex<()>> = Default::default();
        let m3: Arc<Mutex<()>> = Default::default();
        let b = Arc::new(Barrier::new(4));

        let m1_ = m1.clone();
        let m2_ = m2.clone();
        let m3_ = m3.clone();
        let b1 = b.clone();
        let b2 = b.clone();
        let b3 = b.clone();

        assert!(!check_deadlock());

        let _t1 = thread::spawn(move || {
            let _g = m1.lock();
            b1.wait();
            let _ = m2_.lock();
        });

        let _t2 = thread::spawn(move || {
            let _g = m2.lock();
            b2.wait();
            let _ = m3_.lock();
        });

        let _t3 = thread::spawn(move || {
            let _g = m3.lock();
            b3.wait();
            let _ = m1_.lock();
        });

        assert!(!check_deadlock());

        b.wait();
        sleep(Duration::from_millis(50));
        assert!(check_deadlock());

        assert!(!check_deadlock());
    }

    #[test]
    fn test_mutex_deadlock_reentrant() {
        let _guard = DEADLOCK_DETECTION_LOCK.lock();

        let m1: Arc<Mutex<()>> = Default::default();

        assert!(!check_deadlock());

        let _t1 = thread::spawn(move || {
            let _g = m1.lock();
            let _ = m1.lock();
        });

        sleep(Duration::from_millis(50));
        assert!(check_deadlock());

        assert!(!check_deadlock());
    }

    #[test]
    fn test_remutex_deadlock() {
        let _guard = DEADLOCK_DETECTION_LOCK.lock();

        let m1: Arc<ReentrantMutex<()>> = Default::default();
        let m2: Arc<ReentrantMutex<()>> = Default::default();
        let m3: Arc<ReentrantMutex<()>> = Default::default();
        let b = Arc::new(Barrier::new(4));

        let m1_ = m1.clone();
        let m2_ = m2.clone();
        let m3_ = m3.clone();
        let b1 = b.clone();
        let b2 = b.clone();
        let b3 = b.clone();

        assert!(!check_deadlock());

        let _t1 = thread::spawn(move || {
            let _g = m1.lock();
            let _g = m1.lock();
            b1.wait();
            let _ = m2_.lock();
        });

        let _t2 = thread::spawn(move || {
            let _g = m2.lock();
            let _g = m2.lock();
            b2.wait();
            let _ = m3_.lock();
        });

        let _t3 = thread::spawn(move || {
            let _g = m3.lock();
            let _g = m3.lock();
            b3.wait();
            let _ = m1_.lock();
        });

        assert!(!check_deadlock());

        b.wait();
        sleep(Duration::from_millis(50));
        assert!(check_deadlock());

        assert!(!check_deadlock());
    }

    #[test]
    fn test_rwlock_deadlock() {
        let _guard = DEADLOCK_DETECTION_LOCK.lock();

        let m1: Arc<RwLock<()>> = Default::default();
        let m2: Arc<RwLock<()>> = Default::default();
        let m3: Arc<RwLock<()>> = Default::default();
        let b = Arc::new(Barrier::new(4));

        let m1_ = m1.clone();
        let m2_ = m2.clone();
        let m3_ = m3.clone();
        let b1 = b.clone();
        let b2 = b.clone();
        let b3 = b.clone();

        assert!(!check_deadlock());

        let _t1 = thread::spawn(move || {
            let _g = m1.read();
            b1.wait();
            let _g = m2_.write();
        });

        let _t2 = thread::spawn(move || {
            let _g = m2.read();
            b2.wait();
            let _g = m3_.write();
        });

        let _t3 = thread::spawn(move || {
            let _g = m3.read();
            b3.wait();
            let _ = m1_.write();
        });

        assert!(!check_deadlock());

        b.wait();
        sleep(Duration::from_millis(50));
        assert!(check_deadlock());

        assert!(!check_deadlock());
    }

    #[cfg(rwlock_deadlock_detection_not_supported)]
    #[test]
    fn test_rwlock_deadlock_reentrant() {
        let _guard = DEADLOCK_DETECTION_LOCK.lock();

        let m1: Arc<RwLock<()>> = Default::default();

        assert!(!check_deadlock());

        let _t1 = thread::spawn(move || {
            let _g = m1.read();
            let _ = m1.write();
        });

        sleep(Duration::from_millis(50));
        assert!(check_deadlock());

        assert!(!check_deadlock());
    }
}
