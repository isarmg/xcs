#[cfg(test)]
mod tests {
    use std::thread;

    use crate::FxHashMapRand;

    #[test]
    fn cloned_random_states_are_equal() {
        let a = FxHashMapRand::<&str, u32>::default();
        let b = a.clone();

        assert_eq!(a.hasher().seed, b.hasher().seed);
    }

    #[test]
    fn random_states_are_different() {
        let a = FxHashMapRand::<&str, u32>::default();
        let b = FxHashMapRand::<&str, u32>::default();

        // That's the whole point of them being random!
        //
        // N.B.: `FxRandomState` uses a thread-local set to a random value and then incremented,
        //       which means that this is *guaranteed* to pass :>
        assert_ne!(a.hasher().seed, b.hasher().seed);
    }

    #[test]
    fn random_states_are_different_cross_thread() {
        // This is similar to the test above, but uses two different threads, so they both get
        // completely random, unrelated values.
        //
        // This means that this test is technically flaky, but the probability of it failing is
        // `1 / 2.pow(bit_size_of::<usize>())`. Or 1/1.7e19 for 64 bit platforms or 1/4294967295
        // for 32 bit platforms. I suppose this is acceptable.
        let a = FxHashMapRand::<&str, u32>::default();
        let b = thread::spawn(FxHashMapRand::<&str, u32>::default)
            .join()
            .unwrap();

        assert_ne!(a.hasher().seed, b.hasher().seed);
    }
}
