#[cfg(test)]
mod tests {
    use super::{ThreadData, DEFAULT_PARK_TOKEN, DEFAULT_UNPARK_TOKEN};
    use std::{
        ptr,
        sync::{
            atomic::{AtomicIsize, AtomicPtr, AtomicUsize, Ordering},
            Arc,
        },
        thread,
        time::Duration,
    };

    /// Calls a closure for every `ThreadData` currently parked on a given key
    fn for_each(key: usize, mut f: impl FnMut(&ThreadData)) {
        let bucket = super::lock_bucket(key);

        let mut current: *const ThreadData = bucket.queue_head.get();
        while !current.is_null() {
            let current_ref = unsafe { &*current };
            if current_ref.key.load(Ordering::Relaxed) == key {
                f(current_ref);
            }
            current = current_ref.next_in_queue.get();
        }

        // SAFETY: We hold the lock here, as required
        unsafe { bucket.mutex.unlock() };
    }

    macro_rules! test {
        ( $( $name:ident(
            repeats: $repeats:expr,
            latches: $latches:expr,
            delay: $delay:expr,
            threads: $threads:expr,
            single_unparks: $single_unparks:expr);
        )* ) => {
            $(#[test]
            fn $name() {
                let delay = Duration::from_micros($delay);
                for _ in 0..$repeats {
                    run_parking_test($latches, delay, $threads, $single_unparks);
                }
            })*
        };
    }

    test! {
        unpark_all_one_fast(
            repeats: 1000, latches: 1, delay: 0, threads: 1, single_unparks: 0
        );
        unpark_all_hundred_fast(
            repeats: 100, latches: 1, delay: 0, threads: 100, single_unparks: 0
        );
        unpark_one_one_fast(
            repeats: 1000, latches: 1, delay: 0, threads: 1, single_unparks: 1
        );
        unpark_one_hundred_fast(
            repeats: 20, latches: 1, delay: 0, threads: 100, single_unparks: 100
        );
        unpark_one_fifty_then_fifty_all_fast(
            repeats: 50, latches: 1, delay: 0, threads: 100, single_unparks: 50
        );
        unpark_all_one(
            repeats: 100, latches: 1, delay: 10000, threads: 1, single_unparks: 0
        );
        unpark_all_hundred(
            repeats: 100, latches: 1, delay: 10000, threads: 100, single_unparks: 0
        );
        unpark_one_one(
            repeats: 10, latches: 1, delay: 10000, threads: 1, single_unparks: 1
        );
        unpark_one_fifty(
            repeats: 1, latches: 1, delay: 10000, threads: 50, single_unparks: 50
        );
        unpark_one_fifty_then_fifty_all(
            repeats: 2, latches: 1, delay: 10000, threads: 100, single_unparks: 50
        );
        hundred_unpark_all_one_fast(
            repeats: 100, latches: 100, delay: 0, threads: 1, single_unparks: 0
        );
        hundred_unpark_all_one(
            repeats: 1, latches: 100, delay: 10000, threads: 1, single_unparks: 0
        );
    }

    fn run_parking_test(
        num_latches: usize,
        delay: Duration,
        num_threads: usize,
        num_single_unparks: usize,
    ) {
        let mut tests = Vec::with_capacity(num_latches);

        for _ in 0..num_latches {
            let test = Arc::new(SingleLatchTest::new(num_threads));
            let mut threads = Vec::with_capacity(num_threads);
            for _ in 0..num_threads {
                let test = test.clone();
                threads.push(thread::spawn(move || test.run()));
            }
            tests.push((test, threads));
        }

        for unpark_index in 0..num_single_unparks {
            thread::sleep(delay);
            for (test, _) in &tests {
                test.unpark_one(unpark_index);
            }
        }

        for (test, threads) in tests {
            test.finish(num_single_unparks);
            for thread in threads {
                thread.join().expect("Test thread panic");
            }
        }
    }

    struct SingleLatchTest {
        semaphore: AtomicIsize,
        num_awake: AtomicUsize,
        /// Holds the pointer to the last *unprocessed* woken up thread.
        last_awoken: AtomicPtr<ThreadData>,
        /// Total number of threads participating in this test.
        num_threads: usize,
    }

    impl SingleLatchTest {
        pub fn new(num_threads: usize) -> Self {
            Self {
                // This implements a fair (FIFO) semaphore, and it starts out unavailable.
                semaphore: AtomicIsize::new(0),
                num_awake: AtomicUsize::new(0),
                last_awoken: AtomicPtr::new(ptr::null_mut()),
                num_threads,
            }
        }

        pub fn run(&self) {
            // Get one slot from the semaphore
            self.down();

            // Report back to the test verification code that this thread woke up
            let this_thread_ptr = super::with_thread_data(|t| t as *const _ as *mut _);
            self.last_awoken.store(this_thread_ptr, Ordering::SeqCst);
            self.num_awake.fetch_add(1, Ordering::SeqCst);
        }

        pub fn unpark_one(&self, single_unpark_index: usize) {
            // last_awoken should be null at all times except between self.up() and at the bottom
            // of this method where it's reset to null again
            assert!(self.last_awoken.load(Ordering::SeqCst).is_null());

            let mut queue: Vec<*mut ThreadData> = Vec::with_capacity(self.num_threads);
            for_each(self.semaphore_addr(), |thread_data| {
                queue.push(thread_data as *const _ as *mut _);
            });
            assert!(queue.len() <= self.num_threads - single_unpark_index);

            let num_awake_before_up = self.num_awake.load(Ordering::SeqCst);

            self.up();

            // Wait for a parked thread to wake up and update num_awake + last_awoken.
            while self.num_awake.load(Ordering::SeqCst) != num_awake_before_up + 1 {
                thread::yield_now();
            }

            // At this point the other thread should have set last_awoken inside the run() method
            let last_awoken = self.last_awoken.load(Ordering::SeqCst);
            assert!(!last_awoken.is_null());
            if !queue.is_empty() && queue[0] != last_awoken {
                panic!(
                    "Woke up wrong thread:\n\tqueue: {:?}\n\tlast awoken: {:?}",
                    queue, last_awoken
                );
            }
            self.last_awoken.store(ptr::null_mut(), Ordering::SeqCst);
        }

        pub fn finish(&self, num_single_unparks: usize) {
            // The amount of threads not unparked via unpark_one
            let mut num_threads_left = self.num_threads.checked_sub(num_single_unparks).unwrap();

            // Wake remaining threads up with unpark_all. Has to be in a loop, because there might
            // still be threads that has not yet parked.
            while num_threads_left > 0 {
                let mut num_waiting_on_address = 0;
                for_each(self.semaphore_addr(), |_thread_data| {
                    num_waiting_on_address += 1;
                });
                assert!(num_waiting_on_address <= num_threads_left);

                let num_awake_before_unpark = self.num_awake.load(Ordering::SeqCst);

                let num_unparked =
                    unsafe { super::unpark_all(self.semaphore_addr(), DEFAULT_UNPARK_TOKEN) };
                assert!(num_unparked >= num_waiting_on_address);
                assert!(num_unparked <= num_threads_left);

                // Wait for all unparked threads to wake up and update num_awake + last_awoken.
                while self.num_awake.load(Ordering::SeqCst)
                    != num_awake_before_unpark + num_unparked
                {
                    thread::yield_now()
                }

                num_threads_left = num_threads_left.checked_sub(num_unparked).unwrap();
            }
            // By now, all threads should have been woken up
            assert_eq!(self.num_awake.load(Ordering::SeqCst), self.num_threads);

            // Make sure no thread is parked on our semaphore address
            let mut num_waiting_on_address = 0;
            for_each(self.semaphore_addr(), |_thread_data| {
                num_waiting_on_address += 1;
            });
            assert_eq!(num_waiting_on_address, 0);
        }

        pub fn down(&self) {
            let old_semaphore_value = self.semaphore.fetch_sub(1, Ordering::SeqCst);

            if old_semaphore_value > 0 {
                // We acquired the semaphore. Done.
                return;
            }

            // We need to wait.
            let validate = || true;
            let before_sleep = || {};
            let timed_out = |_, _| {};
            unsafe {
                super::park(
                    self.semaphore_addr(),
                    validate,
                    before_sleep,
                    timed_out,
                    DEFAULT_PARK_TOKEN,
                    None,
                );
            }
        }

        pub fn up(&self) {
            let old_semaphore_value = self.semaphore.fetch_add(1, Ordering::SeqCst);

            // Check if anyone was waiting on the semaphore. If they were, then pass ownership to them.
            if old_semaphore_value < 0 {
                // We need to continue until we have actually unparked someone. It might be that
                // the thread we want to pass ownership to has decremented the semaphore counter,
                // but not yet parked.
                loop {
                    match unsafe {
                        super::unpark_one(self.semaphore_addr(), |_| DEFAULT_UNPARK_TOKEN)
                            .unparked_threads
                    } {
                        1 => break,
                        0 => (),
                        i => panic!("Should not wake up {} threads", i),
                    }
                }
            }
        }

        fn semaphore_addr(&self) -> usize {
            &self.semaphore as *const _ as usize
        }
    }
}
