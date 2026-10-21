#[cfg(test)]
mod test {
    use crate::{SeedableRng, RngCore};
    use crate::block::{BlockRng, BlockRng64, BlockRngCore};

    #[derive(Debug, Clone)]
    struct DummyRng {
        counter: u32,
    }

    impl BlockRngCore for DummyRng {
        type Item = u32;

        type Results = [u32; 16];

        fn generate(&mut self, results: &mut Self::Results) {
            for r in results {
                *r = self.counter;
                self.counter = self.counter.wrapping_add(3511615421);
            }
        }
    }

    impl SeedableRng for DummyRng {
        type Seed = [u8; 4];

        fn from_seed(seed: Self::Seed) -> Self {
            DummyRng { counter: u32::from_le_bytes(seed) }
        }
    }

    #[test]
    fn blockrng_next_u32_vs_next_u64() {
        let mut rng1 = BlockRng::<DummyRng>::from_seed([1, 2, 3, 4]);
        let mut rng2 = rng1.clone();
        let mut rng3 = rng1.clone();

        let mut a = [0; 16];
        (&mut a[..4]).copy_from_slice(&rng1.next_u32().to_le_bytes());
        (&mut a[4..12]).copy_from_slice(&rng1.next_u64().to_le_bytes());
        (&mut a[12..]).copy_from_slice(&rng1.next_u32().to_le_bytes());

        let mut b = [0; 16];
        (&mut b[..4]).copy_from_slice(&rng2.next_u32().to_le_bytes());
        (&mut b[4..8]).copy_from_slice(&rng2.next_u32().to_le_bytes());
        (&mut b[8..]).copy_from_slice(&rng2.next_u64().to_le_bytes());
        assert_eq!(a, b);

        let mut c = [0; 16];
        (&mut c[..8]).copy_from_slice(&rng3.next_u64().to_le_bytes());
        (&mut c[8..12]).copy_from_slice(&rng3.next_u32().to_le_bytes());
        (&mut c[12..]).copy_from_slice(&rng3.next_u32().to_le_bytes());
        assert_eq!(a, c);
    }

    #[derive(Debug, Clone)]
    struct DummyRng64 {
        counter: u64,
    }

    impl BlockRngCore for DummyRng64 {
        type Item = u64;

        type Results = [u64; 8];

        fn generate(&mut self, results: &mut Self::Results) {
            for r in results {
                *r = self.counter;
                self.counter = self.counter.wrapping_add(2781463553396133981);
            }
        }
    }

    impl SeedableRng for DummyRng64 {
        type Seed = [u8; 8];

        fn from_seed(seed: Self::Seed) -> Self {
            DummyRng64 { counter: u64::from_le_bytes(seed) }
        }
    }

    #[test]
    fn blockrng64_next_u32_vs_next_u64() {
        let mut rng1 = BlockRng64::<DummyRng64>::from_seed([1, 2, 3, 4, 5, 6, 7, 8]);
        let mut rng2 = rng1.clone();
        let mut rng3 = rng1.clone();

        let mut a = [0; 16];
        (&mut a[..4]).copy_from_slice(&rng1.next_u32().to_le_bytes());
        (&mut a[4..12]).copy_from_slice(&rng1.next_u64().to_le_bytes());
        (&mut a[12..]).copy_from_slice(&rng1.next_u32().to_le_bytes());

        let mut b = [0; 16];
        (&mut b[..4]).copy_from_slice(&rng2.next_u32().to_le_bytes());
        (&mut b[4..8]).copy_from_slice(&rng2.next_u32().to_le_bytes());
        (&mut b[8..]).copy_from_slice(&rng2.next_u64().to_le_bytes());
        assert_ne!(a, b);
        assert_eq!(&a[..4], &b[..4]);
        assert_eq!(&a[4..12], &b[8..]);

        let mut c = [0; 16];
        (&mut c[..8]).copy_from_slice(&rng3.next_u64().to_le_bytes());
        (&mut c[8..12]).copy_from_slice(&rng3.next_u32().to_le_bytes());
        (&mut c[12..]).copy_from_slice(&rng3.next_u32().to_le_bytes());
        assert_eq!(b, c);
    }
}
