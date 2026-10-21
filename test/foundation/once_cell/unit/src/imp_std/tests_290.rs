#[cfg(test)]
mod tests {
    use std::panic;
    use std::{sync::mpsc::channel, thread};

    use super::OnceCell;

    impl<T> OnceCell<T> {
        fn init(&self, f: impl FnOnce() -> T) {
            enum Void {}
            let _ = self.initialize(|| Ok::<T, Void>(f()));
        }
    }

    #[test]
    fn smoke_once() {
        static O: OnceCell<()> = OnceCell::new();
        let mut a = 0;
        O.init(|| a += 1);
        assert_eq!(a, 1);
        O.init(|| a += 1);
        assert_eq!(a, 1);
    }

    #[test]
    fn stampede_once() {
        static O: OnceCell<()> = OnceCell::new();
        static mut RUN: bool = false;

        let (tx, rx) = channel();
        for _ in 0..10 {
            let tx = tx.clone();
            thread::spawn(move || {
                for _ in 0..4 {
                    thread::yield_now()
                }
                unsafe {
                    O.init(|| {
                        assert!(!RUN);
                        RUN = true;
                    });
                    assert!(RUN);
                }
                tx.send(()).unwrap();
            });
        }

        unsafe {
            O.init(|| {
                assert!(!RUN);
                RUN = true;
            });
            assert!(RUN);
        }

        for _ in 0..10 {
            rx.recv().unwrap();
        }
    }

    #[test]
    fn poison_bad() {
        static O: OnceCell<()> = OnceCell::new();

        // poison the once
        let t = panic::catch_unwind(|| {
            O.init(|| panic!());
        });
        assert!(t.is_err());

        // we can subvert poisoning, however
        let mut called = false;
        O.init(|| {
            called = true;
        });
        assert!(called);

        // once any success happens, we stop propagating the poison
        O.init(|| {});
    }

    #[test]
    fn wait_for_force_to_finish() {
        static O: OnceCell<()> = OnceCell::new();

        // poison the once
        let t = panic::catch_unwind(|| {
            O.init(|| panic!());
        });
        assert!(t.is_err());

        // make sure someone's waiting inside the once via a force
        let (tx1, rx1) = channel();
        let (tx2, rx2) = channel();
        let t1 = thread::spawn(move || {
            O.init(|| {
                tx1.send(()).unwrap();
                rx2.recv().unwrap();
            });
        });

        rx1.recv().unwrap();

        // put another waiter on the once
        let t2 = thread::spawn(|| {
            let mut called = false;
            O.init(|| {
                called = true;
            });
            assert!(!called);
        });

        tx2.send(()).unwrap();

        assert!(t1.join().is_ok());
        assert!(t2.join().is_ok());
    }

    #[test]
    #[cfg(target_pointer_width = "64")]
    fn test_size() {
        use std::mem::size_of;

        assert_eq!(size_of::<OnceCell<u32>>(), 4 * size_of::<u32>());
    }
}
