#[cfg(test)]
mod tests {
    #[cfg(not(any(target_pointer_width = "64", target_pointer_width = "32")))]
    compile_error!("The test suite only supports 64 bit and 32 bit usize");

    use crate::{FxBuildHasher, FxHasher};
    use core::hash::{BuildHasher, Hash, Hasher};

    macro_rules! test_hash {
        (
            $(
                hash($value:expr) == $result:expr,
            )*
        ) => {
            $(
                assert_eq!(FxBuildHasher.hash_one($value), $result);
            )*
        };
    }

    const B32: bool = cfg!(target_pointer_width = "32");

    #[test]
    fn unsigned() {
        test_hash! {
            hash(0_u8) == 0,
            hash(1_u8) == if B32 { 3001993707 } else { 12157901119326311915 },
            hash(100_u8) == if B32 { 3844759569 } else { 16751747135202103309 },
            hash(u8::MAX) == if B32 { 999399879 } else { 1211781028898739645 },

            hash(0_u16) == 0,
            hash(1_u16) == if B32 { 3001993707 } else { 12157901119326311915 },
            hash(100_u16) == if B32 { 3844759569 } else { 16751747135202103309 },
            hash(u16::MAX) == if B32 { 3440503042 } else { 16279819243059860173 },

            hash(0_u32) == 0,
            hash(1_u32) == if B32 { 3001993707 } else { 12157901119326311915 },
            hash(100_u32) == if B32 { 3844759569 } else { 16751747135202103309 },
            hash(u32::MAX) == if B32 { 1293006356 } else { 7729994835221066939 },

            hash(0_u64) == 0,
            hash(1_u64) == if B32 { 275023839 } else { 12157901119326311915 },
            hash(100_u64) == if B32 { 1732383522 } else { 16751747135202103309 },
            hash(u64::MAX) == if B32 { 1017982517 } else { 6288842954450348564 },

            hash(0_u128) == 0,
            hash(1_u128) == if B32 { 1860738631 } else { 13032756267696824044 },
            hash(100_u128) == if B32 { 1389515751 } else { 12003541609544029302 },
            hash(u128::MAX) == if B32 { 2156022013 } else { 11702830760530184999 },

            hash(0_usize) == 0,
            hash(1_usize) == if B32 { 3001993707 } else { 12157901119326311915 },
            hash(100_usize) == if B32 { 3844759569 } else { 16751747135202103309 },
            hash(usize::MAX) == if B32 { 1293006356 } else { 6288842954450348564 },
        }
    }

    #[test]
    fn signed() {
        test_hash! {
            hash(i8::MIN) == if B32 { 2000713177 } else { 6684841074112525780 },
            hash(0_i8) == 0,
            hash(1_i8) == if B32 { 3001993707 } else { 12157901119326311915 },
            hash(100_i8) == if B32 { 3844759569 } else { 16751747135202103309 },
            hash(i8::MAX) == if B32 { 3293686765 } else { 12973684028562874344 },

            hash(i16::MIN) == if B32 { 1073764727 } else { 14218860181193086044 },
            hash(0_i16) == 0,
            hash(1_i16) == if B32 { 3001993707 } else { 12157901119326311915 },
            hash(100_i16) == if B32 { 3844759569 } else { 16751747135202103309 },
            hash(i16::MAX) == if B32 { 2366738315 } else { 2060959061933882993 },

            hash(i32::MIN) == if B32 { 16384 } else { 9943947977240134995 },
            hash(0_i32) == 0,
            hash(1_i32) == if B32 { 3001993707 } else { 12157901119326311915 },
            hash(100_i32) == if B32 { 3844759569 } else { 16751747135202103309 },
            hash(i32::MAX) == if B32 { 1293022740 } else { 16232790931690483559 },

            hash(i64::MIN) == if B32 { 16384 } else { 33554432 },
            hash(0_i64) == 0,
            hash(1_i64) == if B32 { 275023839 } else { 12157901119326311915 },
            hash(100_i64) == if B32 { 1732383522 } else { 16751747135202103309 },
            hash(i64::MAX) == if B32 { 1017998901 } else { 6288842954483902996 },

            hash(i128::MIN) == if B32 { 16384 } else { 33554432 },
            hash(0_i128) == 0,
            hash(1_i128) == if B32 { 1860738631 } else { 13032756267696824044 },
            hash(100_i128) == if B32 { 1389515751 } else { 12003541609544029302 },
            hash(i128::MAX) == if B32 { 2156005629 } else { 11702830760496630567 },

            hash(isize::MIN) == if B32 { 16384 } else { 33554432 },
            hash(0_isize) == 0,
            hash(1_isize) == if B32 { 3001993707 } else { 12157901119326311915 },
            hash(100_isize) == if B32 { 3844759569 } else { 16751747135202103309 },
            hash(isize::MAX) == if B32 { 1293022740 } else { 6288842954483902996 },
        }
    }

    // Avoid relying on any `Hash` implementations in the standard library.
    struct HashBytes(&'static [u8]);
    impl Hash for HashBytes {
        fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
            state.write(self.0);
        }
    }

    #[test]
    fn bytes() {
        test_hash! {
            hash(HashBytes(&[])) == if B32 { 2673204745 } else { 17606491139363777937 },
            hash(HashBytes(&[0])) == if B32 { 2948228584 } else { 5448590020104574886 },
            hash(HashBytes(&[0, 0, 0, 0, 0, 0])) == if B32 { 3223252423 } else { 16766921560080789783 },
            hash(HashBytes(&[1])) == if B32 { 2943445104 } else { 5922447956811044110 },
            hash(HashBytes(&[2])) == if B32 { 1055423297 } else { 5229781508510959783 },
            hash(HashBytes(b"uwu")) == if B32 { 2699662140 } else { 7168164714682931527 },
            hash(HashBytes(b"These are some bytes for testing rustc_hash.")) == if B32 { 2303640537 } else { 2349210501944688211 },
        }
    }

    #[test]
    fn with_seed_actually_different() {
        let seeds = [
            [1, 2],
            [42, 17],
            [124436707, 99237],
            [usize::MIN, usize::MAX],
        ];

        for [a_seed, b_seed] in seeds {
            let a = || FxHasher::with_seed(a_seed);
            let b = || FxHasher::with_seed(b_seed);

            for x in u8::MIN..=u8::MAX {
                let mut a = a();
                let mut b = b();

                x.hash(&mut a);
                x.hash(&mut b);

                assert_ne!(a.finish(), b.finish())
            }
        }
    }
}
