#[cfg(test)]
pub(crate) mod testutil {
    use crate::*;

    /// A `T` which is aligned to at least `align_of::<A>()`.
    #[derive(Default)]
    pub(crate) struct Align<T, A> {
        pub(crate) t: T,
        _a: [A; 0],
    }

    impl<T: Default, A> Align<T, A> {
        pub(crate) fn set_default(&mut self) {
            self.t = T::default();
        }
    }

    impl<T, A> Align<T, A> {
        pub(crate) const fn new(t: T) -> Align<T, A> {
            Align { t, _a: [] }
        }
    }

    /// A `T` which is guaranteed not to satisfy `align_of::<A>()`.
    ///
    /// It must be the case that `align_of::<T>() < align_of::<A>()` in order
    /// for this type to work properly.
    #[repr(C)]
    pub(crate) struct ForceUnalign<T: Unaligned, A> {
        // The outer struct is aligned to `A`, and, thanks to `repr(C)`, `t` is
        // placed at the minimum offset that guarantees its alignment. If
        // `align_of::<T>() < align_of::<A>()`, then that offset will be
        // guaranteed *not* to satisfy `align_of::<A>()`.
        //
        // Note that we need `T: Unaligned` in order to guarantee that there is
        // no padding between `_u` and `t`.
        _u: u8,
        pub(crate) t: T,
        _a: [A; 0],
    }

    impl<T: Unaligned, A> ForceUnalign<T, A> {
        pub(crate) fn new(t: T) -> ForceUnalign<T, A> {
            ForceUnalign { _u: 0, t, _a: [] }
        }
    }
    // A `u64` with alignment 8.
    //
    // Though `u64` has alignment 8 on some platforms, it's not guaranteed. By
    // contrast, `AU64` is guaranteed to have alignment 8 on all platforms.
    #[derive(
        KnownLayout,
        Immutable,
        FromBytes,
        IntoBytes,
        Eq,
        PartialEq,
        Ord,
        PartialOrd,
        Default,
        Debug,
        Copy,
        Clone,
    )]
    #[repr(C, align(8))]
    pub(crate) struct AU64(pub(crate) u64);

    impl AU64 {
        // Converts this `AU64` to bytes using this platform's endianness.
        pub(crate) fn to_bytes(self) -> [u8; 8] {
            crate::transmute!(self)
        }
    }

    impl Display for AU64 {
        #[cfg_attr(
            all(coverage_nightly, __ZEROCOPY_INTERNAL_USE_ONLY_NIGHTLY_FEATURES_IN_TESTS),
            coverage(off)
        )]
        fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
            Display::fmt(&self.0, f)
        }
    }
}
