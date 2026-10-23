//! Development-related functionality

#![allow(clippy::missing_errors_doc)]

mod fixed;
#[cfg(feature = "mac")]
mod mac;
mod rng;
mod xof;

pub use fixed::*;
#[cfg(feature = "mac")]
pub use mac::*;
pub use xof::*;

/// Test vector for hash functions
#[derive(Debug, Clone, Copy)]
pub struct TestVector {
    /// Input data
    pub input: &'static [u8],
    /// Output hash
    pub output: &'static [u8],
}

/// Define hash function serialization test
#[macro_export]
macro_rules! hash_serialization_test {
    ($name:ident, $hasher:ty $(,)?) => {
        #[test]
        fn $name() {
            use cryptographic_core::{
                Digest,
                common_types::{BlockSizeUser, hazmat::SerializableState},
                typenum::Unsigned,
            };

            let mut h = <$hasher>::new();

            h.update(&[0x13; <$hasher as BlockSizeUser>::BlockSize::USIZE + 1]);

            let serialized_state = h.serialize();
            let expected = include_bytes!(concat!("data/", stringify!($name), ".bin"));
            assert_eq!(serialized_state.as_slice(), expected);

            let mut h = <$hasher>::deserialize(&serialized_state).unwrap();

            h.update(&[0x13; <$hasher as BlockSizeUser>::BlockSize::USIZE + 1]);
            let output1 = h.finalize();

            let mut h = <$hasher>::new();
            h.update(&[0x13; 2 * (<$hasher as BlockSizeUser>::BlockSize::USIZE + 1)]);
            let output2 = h.finalize();

            assert_eq!(output1, output2);
        }
    };
}

/// Define [`Update`][crate::Update] impl benchmark
#[macro_export]
macro_rules! bench_update {
    (
        $init:expr;
        $($name:ident $bs:expr;)*
    ) => {
        $(
            #[bench]
            fn $name(b: &mut Bencher) {
                let mut d = $init;
                let data = [0; $bs];

                b.iter(|| {
                    cryptographic_core::Update::update(&mut d, &data[..]);
                });

                b.bytes = $bs;
            }
        )*
    };
}

/// Feed ~16 MiB of pseudorandom data to an updatable state.
pub fn feed_rand_16mib<D: crate::Update>(d: &mut D) {
    let buf = &mut [0u8; 1024];
    let mut rng = rng::RNG;
    let n = 16 * (1 << 20) / buf.len();
    for _ in 0..n {
        rng.fill(buf);
        d.update(buf);
        // additional byte, so size of fed data
        // will not be multiple of block size
        d.update(&[42]);
    }
}
