#[cfg(all(test, __ZEROCOPY_INTERNAL_USE_ONLY_NIGHTLY_FEATURES_IN_TESTS))]
mod benches {
    use test::{self, Bencher};

    use super::*;
    use crate::util::testutil::*;

    #[bench]
    fn bench_from_bytes_sized(b: &mut Bencher) {
        let buf = Align::<[u8; 8], AU64>::default();
        // `buf.t` should be aligned to 8, so this should always succeed.
        let bytes = &buf.t[..];
        b.iter(|| test::black_box(Ref::<_, AU64>::from_bytes(test::black_box(bytes)).unwrap()));
    }

    #[bench]
    fn bench_into_ref_sized(b: &mut Bencher) {
        let buf = Align::<[u8; 8], AU64>::default();
        let bytes = &buf.t[..];
        let r = Ref::<_, AU64>::from_bytes(bytes).unwrap();
        b.iter(|| test::black_box(Ref::into_ref(test::black_box(r))));
    }

    #[bench]
    fn bench_into_mut_sized(b: &mut Bencher) {
        let mut buf = Align::<[u8; 8], AU64>::default();
        let buf = &mut buf.t[..];
        let _ = Ref::<_, AU64>::from_bytes(&mut *buf).unwrap();
        b.iter(move || {
            // SAFETY: The preceding `from_bytes` succeeded, and so we know that
            // `buf` is validly aligned and has the `size_of::<AU64>()` byte
            // length required by `new_sized_unchecked` [1].
            //
            // [1] Per https://doc.rust-lang.org/1.56.0/core/mem/fn.size_of.html:
            //
            //     Returns the size of a type in bytes.
            let r = unsafe { Ref::<&mut [u8], AU64>::new_sized_unchecked(&mut *buf) };
            test::black_box(Ref::into_mut(test::black_box(r)));
        });
    }

    #[bench]
    fn bench_deref_sized(b: &mut Bencher) {
        let buf = Align::<[u8; 8], AU64>::default();
        let bytes = &buf.t[..];
        let r = Ref::<_, AU64>::from_bytes(bytes).unwrap();
        b.iter(|| {
            let temp = test::black_box(r);
            test::black_box(temp.deref());
        });
    }

    #[bench]
    fn bench_deref_mut_sized(b: &mut Bencher) {
        let mut buf = Align::<[u8; 8], AU64>::default();
        let buf = &mut buf.t[..];
        let _ = Ref::<_, AU64>::from_bytes(&mut *buf).unwrap();
        b.iter(|| {
            // SAFETY: The preceding `from_bytes` succeeded, and so we know that
            // `buf` is validly aligned and has the `size_of::<AU64>()` byte
            // length required by `new_sized_unchecked` [1].
            //
            // [1] Per https://doc.rust-lang.org/1.56.0/core/mem/fn.size_of.html:
            //
            //     Returns the size of a type in bytes.
            let r = unsafe { Ref::<&mut [u8], AU64>::new_sized_unchecked(&mut *buf) };
            let mut temp = test::black_box(r);
            test::black_box(temp.deref_mut());
        });
    }
}
