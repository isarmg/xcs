#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_round_down_to_next_multiple_of_alignment() {
        // Division by `NonZeroUsize` unsupported on our MSRV.
        #[allow(clippy::needless_nonzero_get)]
        fn alt_impl(n: usize, align: NonZeroUsize) -> usize {
            let mul = n / align.get();
            mul * align.get()
        }

        for align in [1, 2, 4, 8, 16] {
            for n in 0..256 {
                let align = NonZeroUsize::new(align).unwrap();
                let want = alt_impl(n, align);
                let got = round_down_to_next_multiple_of_alignment(n, align);
                assert_eq!(got, want, "round_down_to_next_multiple_of_alignment({}, {})", n, align);
            }
        }
    }

    #[rustversion::since(1.57.0)]
    #[test]
    #[should_panic]
    fn test_round_down_to_next_multiple_of_alignment_zerocopy_panic_in_const_and_vec_try_reserve() {
        round_down_to_next_multiple_of_alignment(0, NonZeroUsize::new(3).unwrap());
    }
    #[test]
    fn test_send_sync_phantom_data() {
        let x = SendSyncPhantomData::<u8>::default();
        let y = x.clone();
        assert!(x == y);
        assert!(x == SendSyncPhantomData::<u8>::default());
    }

    #[test]
    #[allow(clippy::as_conversions)]
    fn test_as_address() {
        let x = 0u8;
        let r = &x;
        let mut x_mut = 0u8;
        let rm = &mut x_mut;
        let p = r as *const u8;
        let pm = rm as *mut u8;
        let nn = NonNull::new(p as *mut u8).unwrap();

        assert_eq!(AsAddress::addr(r), p as usize);
        assert_eq!(AsAddress::addr(rm), pm as usize);
        assert_eq!(AsAddress::addr(p), p as usize);
        assert_eq!(AsAddress::addr(pm), pm as usize);
        assert_eq!(AsAddress::addr(nn), p as usize);
    }
}
