#[cfg(test)]
mod test {
    use super::*;
    use crate::Pack;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn addr_roundtrips(pidx in 0usize..Addr::<cfg::DefaultConfig>::BITS) {
            let addr = Addr::<cfg::DefaultConfig>::from_usize(pidx);
            let packed = addr.pack(0);
            assert_eq!(addr, Addr::from_packed(packed));
        }
        #[test]
        fn gen_roundtrips(gen in 0usize..slot::Generation::<cfg::DefaultConfig>::BITS) {
            let gen = slot::Generation::<cfg::DefaultConfig>::from_usize(gen);
            let packed = gen.pack(0);
            assert_eq!(gen, slot::Generation::from_packed(packed));
        }

        #[test]
        fn page_roundtrips(
            gen in 0usize..slot::Generation::<cfg::DefaultConfig>::BITS,
            addr in 0usize..Addr::<cfg::DefaultConfig>::BITS,
        ) {
            let gen = slot::Generation::<cfg::DefaultConfig>::from_usize(gen);
            let addr = Addr::<cfg::DefaultConfig>::from_usize(addr);
            let packed = gen.pack(addr.pack(0));
            assert_eq!(addr, Addr::from_packed(packed));
            assert_eq!(gen, slot::Generation::from_packed(packed));
        }
    }
}
