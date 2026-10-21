#[cfg(test)]
mod tests {
    use super::*;
    use std::rc;
    use std::sync;

    #[test]
    fn arc_rw_read() {
        let base = sync::Arc::new(sync::RwLock::new(true));

        // the use of scopes below is necessary so that we can drop base at the end.
        // otherwise, all the x1's (i.e., base.read()) would hold on to borrows.
        // this is part of the problem that Guardian is trying to solve.

        let x2 = {
            let x1 = base.read().unwrap();
            let x2 = ArcRwLockReadGuardian::take(base.clone()).unwrap();

            // guardian dereferences correctly
            assert_eq!(&*x1, &*x2);

            // guardian holds read lock
            drop(x1);
            assert!(base.try_write().is_err(), "guardian holds read lock");

            x2
        };

        {
            // guardian can be moved
            let x1 = base.read().unwrap();
            let x2_ = x2;
            assert_eq!(&*x1, &*x2_);

            // moving guardian does not release lock
            drop(x1);
            assert!(base.try_write().is_err(), "guardian still holds read lock");

            // dropping guardian drops read lock
            drop(x2_);
            assert!(base.try_write().is_ok(), "guardian drops read lock");
        }

        // guardian works even after all other Arcs have been dropped
        let x = ArcRwLockReadGuardian::take(base).unwrap();
        assert_eq!(&*x, &true);
    }

    #[test]
    fn arc_rw_write() {
        let base = sync::Arc::new(sync::RwLock::new(true));

        let mut x = ArcRwLockWriteGuardian::take(base.clone()).unwrap();

        // guardian dereferences correctly
        assert_eq!(&*x, &true);

        // guardian can write
        *x = false;
        assert_eq!(&*x, &false);

        // guardian holds write lock
        assert!(base.try_read().is_err(), "guardian holds write lock");

        // guardian can be moved
        let x_ = x;
        assert_eq!(&*x_, &false);

        // moving guardian does not release lock
        assert!(base.try_read().is_err(), "guardian still holds write lock");

        // dropping guardian drops write lock
        drop(x_);
        assert!(base.try_read().is_ok(), "guardian drops write lock");

        // guardian works even after all other Arcs have been dropped
        let x = ArcRwLockWriteGuardian::take(base).unwrap();
        assert_eq!(&*x, &false);
    }

    #[test]
    fn arc_rw_try() {
        let base = sync::Arc::new(sync::RwLock::new(true));

        let mut x = ArcRwLockWriteGuardian::try_take(base.clone())
            .unwrap()
            .unwrap();

        // guardian dereferences correctly
        assert_eq!(&*x, &true);

        // guardian can write
        *x = false;
        assert_eq!(&*x, &false);

        // guardian holds write lock
        assert!(base.try_read().is_err(), "guardian holds write lock");

        // guardian can be moved
        let x_ = x;
        assert_eq!(&*x_, &false);

        // moving guardian does not release lock
        assert!(base.try_read().is_err(), "guardian still holds write lock");

        // try_take returns None if it would block
        assert!(ArcRwLockWriteGuardian::try_take(base.clone()).is_none());

        assert!(ArcRwLockReadGuardian::try_take(base.clone()).is_none());

        // dropping guardian drops write lock
        drop(x_);
        assert!(base.try_read().is_ok(), "guardian drops write lock");

        // guardian works even after all other Arcs have been dropped
        let x = ArcRwLockWriteGuardian::take(base).unwrap();
        assert_eq!(&*x, &false);
    }

    #[test]
    fn arc_mu() {
        let base = sync::Arc::new(sync::Mutex::new(true));

        let mut x = ArcMutexGuardian::take(base.clone()).unwrap();

        // guardian dereferences correctly
        assert_eq!(&*x, &true);

        // guardian can write
        *x = false;
        assert_eq!(&*x, &false);

        // guardian holds lock
        assert!(base.try_lock().is_err(), "guardian holds lock");

        // guardian can be moved
        let x_ = x;
        assert_eq!(&*x_, &false);

        // moving guardian does not release lock
        assert!(base.try_lock().is_err(), "guardian still holds lock");

        // dropping guardian drops lock
        drop(x_);
        assert!(base.try_lock().is_ok(), "guardian drops lock");

        // guardian works even after all other Arcs have been dropped
        let x = ArcMutexGuardian::take(base).unwrap();
        assert_eq!(&*x, &false);
    }

    #[test]
    fn arc_mu_try() {
        let base = sync::Arc::new(sync::Mutex::new(true));

        let mut x = ArcMutexGuardian::try_take(base.clone()).unwrap().unwrap();

        // guardian dereferences correctly
        assert_eq!(&*x, &true);

        // guardian can write
        *x = false;
        assert_eq!(&*x, &false);

        // guardian holds lock
        assert!(base.try_lock().is_err(), "guardian holds lock");

        // guardian can be moved
        let x_ = x;
        assert_eq!(&*x_, &false);

        // moving guardian does not release lock
        assert!(base.try_lock().is_err(), "guardian still holds lock");

        // try_take returns None if it would block
        assert!(ArcMutexGuardian::try_take(base.clone()).is_none());

        // dropping guardian drops lock
        drop(x_);
        assert!(base.try_lock().is_ok(), "guardian drops lock");

        // guardian works even after all other Arcs have been dropped
        let x = ArcMutexGuardian::take(base).unwrap();
        assert_eq!(&*x, &false);
    }

    #[test]
    fn rc_rw_read() {
        let base = rc::Rc::new(sync::RwLock::new(true));

        // the use of scopes below is necessary so that we can drop base at the end.
        // otherwise, all the x1's (i.e., base.read()) would hold on to borrows.
        // this is part of the problem that Guardian is trying to solve.

        let x2 = {
            let x1 = base.read().unwrap();
            let x2 = RcRwLockReadGuardian::take(base.clone()).unwrap();

            // guardian dereferences correctly
            assert_eq!(&*x1, &*x2);

            // guardian holds read lock
            drop(x1);
            assert!(base.try_write().is_err(), "guardian holds read lock");

            x2
        };

        {
            // guardian can be moved
            let x1 = base.read().unwrap();
            let x2_ = x2;
            assert_eq!(&*x1, &*x2_);

            // moving guardian does not release lock
            drop(x1);
            assert!(base.try_write().is_err(), "guardian still holds read lock");

            // dropping guardian drops read lock
            drop(x2_);
            assert!(base.try_write().is_ok(), "guardian drops read lock");
        }

        // guardian works even after all other Rcs have been dropped
        let x = RcRwLockReadGuardian::take(base).unwrap();
        assert_eq!(&*x, &true);
    }

    #[test]
    fn rc_rw_write() {
        let base = rc::Rc::new(sync::RwLock::new(true));

        let mut x = RcRwLockWriteGuardian::take(base.clone()).unwrap();

        // guardian dereferences correctly
        assert_eq!(&*x, &true);

        // guardian can write
        *x = false;
        assert_eq!(&*x, &false);

        // guardian holds write lock
        assert!(base.try_read().is_err(), "guardian holds write lock");

        // guardian can be moved
        let x_ = x;
        assert_eq!(&*x_, &false);

        // moving guardian does not release lock
        assert!(base.try_read().is_err(), "guardian still holds write lock");

        // dropping guardian drops write lock
        drop(x_);
        assert!(base.try_read().is_ok(), "guardian drops write lock");

        // guardian works even after all other Rcs have been dropped
        let x = RcRwLockWriteGuardian::take(base).unwrap();
        assert_eq!(&*x, &false);
    }

    #[test]
    fn rc_rw_try() {
        let base = rc::Rc::new(sync::RwLock::new(true));

        let mut x = RcRwLockWriteGuardian::try_take(base.clone())
            .unwrap()
            .unwrap();

        // guardian dereferences correctly
        assert_eq!(&*x, &true);

        // guardian can write
        *x = false;
        assert_eq!(&*x, &false);

        // guardian holds write lock
        assert!(base.try_read().is_err(), "guardian holds write lock");

        // guardian can be moved
        let x_ = x;
        assert_eq!(&*x_, &false);

        // moving guardian does not release lock
        assert!(base.try_read().is_err(), "guardian still holds write lock");

        // try_take returns None if it would block
        assert!(RcRwLockWriteGuardian::try_take(base.clone()).is_none());

        assert!(RcRwLockReadGuardian::try_take(base.clone()).is_none());

        // dropping guardian drops write lock
        drop(x_);
        assert!(base.try_read().is_ok(), "guardian drops write lock");

        // guardian works even after all other Rcs have been dropped
        let x = RcRwLockWriteGuardian::take(base).unwrap();
        assert_eq!(&*x, &false);
    }

    #[test]
    fn rc_mu() {
        let base = rc::Rc::new(sync::Mutex::new(true));

        let mut x = RcMutexGuardian::take(base.clone()).unwrap();

        // guardian dereferences correctly
        assert_eq!(&*x, &true);

        // guardian can write
        *x = false;
        assert_eq!(&*x, &false);

        // guardian holds lock
        assert!(base.try_lock().is_err(), "guardian holds lock");

        // guardian can be moved
        let x_ = x;
        assert_eq!(&*x_, &false);

        // moving guardian does not release lock
        assert!(base.try_lock().is_err(), "guardian still holds lock");

        // dropping guardian drops lock
        drop(x_);
        assert!(base.try_lock().is_ok(), "guardian drops lock");

        // guardian works even after all other Rcs have been dropped
        let x = RcMutexGuardian::take(base).unwrap();
        assert_eq!(&*x, &false);
    }

    #[test]
    fn rc_mu_try() {
        let base = rc::Rc::new(sync::Mutex::new(true));

        let mut x = RcMutexGuardian::take(base.clone()).unwrap();

        // guardian dereferences correctly
        assert_eq!(&*x, &true);

        // guardian can write
        *x = false;
        assert_eq!(&*x, &false);

        // guardian holds lock
        assert!(base.try_lock().is_err(), "guardian holds lock");

        // guardian can be moved
        let x_ = x;
        assert_eq!(&*x_, &false);

        // moving guardian does not release lock
        assert!(base.try_lock().is_err(), "guardian still holds lock");

        // try_take returns None if it would block
        assert!(RcMutexGuardian::try_take(base.clone()).is_none());

        // dropping guardian drops lock
        drop(x_);
        assert!(base.try_lock().is_ok(), "guardian drops lock");

        // guardian works even after all other Rcs have been dropped
        let x = RcMutexGuardian::take(base).unwrap();
        assert_eq!(&*x, &false);
    }
}
