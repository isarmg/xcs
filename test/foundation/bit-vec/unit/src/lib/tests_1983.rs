#[cfg(test)]
mod tests {
    use super::{BitVec, Iter, Vec};

    // This is stupid, but I want to differentiate from a "random" 32
    const U32_BITS: usize = 32;

    #[test]
    fn test_display_output() {
        assert_eq!(format!("{}", BitVec::new()), "");
        assert_eq!(format!("{}", BitVec::from_elem(1, true)), "1");
        assert_eq!(format!("{}", BitVec::from_elem(8, false)), "00000000")
    }

    #[test]
    fn test_debug_output() {
        assert_eq!(
            format!("{:?}", BitVec::new()),
            "BitVec { storage: \"\", nbits: 0 }"
        );
        assert_eq!(
            format!("{:?}", BitVec::from_elem(1, true)),
            "BitVec { storage: \"1\", nbits: 1 }"
        );
        assert_eq!(
            format!("{:?}", BitVec::from_elem(8, false)),
            "BitVec { storage: \"00000000\", nbits: 8 }"
        );
        assert_eq!(
            format!("{:?}", BitVec::from_elem(33, true)),
            "BitVec { storage: \"11111111111111111111111111111111 1\", nbits: 33 }"
        );
        assert_eq!(
            format!(
                "{:?}",
                BitVec::from_bytes(&[0b111, 0b000, 0b1110, 0b0001, 0b11111111, 0b00000000])
            ),
            "BitVec { storage: \"00000111000000000000111000000001 1111111100000000\", nbits: 48 }"
        )
    }

    #[test]
    fn test_0_elements() {
        let act = BitVec::new();
        let exp = Vec::new();
        assert!(act.eq_vec(&exp));
        assert!(act.none() && act.all());
    }

    #[test]
    fn test_1_element() {
        let mut act = BitVec::from_elem(1, false);
        assert!(act.eq_vec(&[false]));
        assert!(act.none() && !act.all());
        act = BitVec::from_elem(1, true);
        assert!(act.eq_vec(&[true]));
        assert!(!act.none() && act.all());
    }

    #[test]
    fn test_2_elements() {
        let mut b = BitVec::from_elem(2, false);
        b.set(0, true);
        b.set(1, false);
        assert_eq!(format!("{}", b), "10");
        assert!(!b.none() && !b.all());
    }

    #[test]
    fn test_10_elements() {
        // all 0

        let mut act = BitVec::from_elem(10, false);
        assert!(
            (act.eq_vec(&[false, false, false, false, false, false, false, false, false, false]))
        );
        assert!(act.none() && !act.all());
        // all 1

        act = BitVec::from_elem(10, true);
        assert!((act.eq_vec(&[true, true, true, true, true, true, true, true, true, true])));
        assert!(!act.none() && act.all());
        // mixed

        act = BitVec::from_elem(10, false);
        act.set(0, true);
        act.set(1, true);
        act.set(2, true);
        act.set(3, true);
        act.set(4, true);
        assert!((act.eq_vec(&[true, true, true, true, true, false, false, false, false, false])));
        assert!(!act.none() && !act.all());
        // mixed

        act = BitVec::from_elem(10, false);
        act.set(5, true);
        act.set(6, true);
        act.set(7, true);
        act.set(8, true);
        act.set(9, true);
        assert!((act.eq_vec(&[false, false, false, false, false, true, true, true, true, true])));
        assert!(!act.none() && !act.all());
        // mixed

        act = BitVec::from_elem(10, false);
        act.set(0, true);
        act.set(3, true);
        act.set(6, true);
        act.set(9, true);
        assert!((act.eq_vec(&[true, false, false, true, false, false, true, false, false, true])));
        assert!(!act.none() && !act.all());
    }

    #[test]
    fn test_31_elements() {
        // all 0

        let mut act = BitVec::from_elem(31, false);
        assert!(act.eq_vec(&[
            false, false, false, false, false, false, false, false, false, false, false, false,
            false, false, false, false, false, false, false, false, false, false, false, false,
            false, false, false, false, false, false, false
        ]));
        assert!(act.none() && !act.all());
        // all 1

        act = BitVec::from_elem(31, true);
        assert!(act.eq_vec(&[
            true, true, true, true, true, true, true, true, true, true, true, true, true, true,
            true, true, true, true, true, true, true, true, true, true, true, true, true, true,
            true, true, true
        ]));
        assert!(!act.none() && act.all());
        // mixed

        act = BitVec::from_elem(31, false);
        act.set(0, true);
        act.set(1, true);
        act.set(2, true);
        act.set(3, true);
        act.set(4, true);
        act.set(5, true);
        act.set(6, true);
        act.set(7, true);
        assert!(act.eq_vec(&[
            true, true, true, true, true, true, true, true, false, false, false, false, false,
            false, false, false, false, false, false, false, false, false, false, false, false,
            false, false, false, false, false, false
        ]));
        assert!(!act.none() && !act.all());
        // mixed

        act = BitVec::from_elem(31, false);
        act.set(16, true);
        act.set(17, true);
        act.set(18, true);
        act.set(19, true);
        act.set(20, true);
        act.set(21, true);
        act.set(22, true);
        act.set(23, true);
        assert!(act.eq_vec(&[
            false, false, false, false, false, false, false, false, false, false, false, false,
            false, false, false, false, true, true, true, true, true, true, true, true, false,
            false, false, false, false, false, false
        ]));
        assert!(!act.none() && !act.all());
        // mixed

        act = BitVec::from_elem(31, false);
        act.set(24, true);
        act.set(25, true);
        act.set(26, true);
        act.set(27, true);
        act.set(28, true);
        act.set(29, true);
        act.set(30, true);
        assert!(act.eq_vec(&[
            false, false, false, false, false, false, false, false, false, false, false, false,
            false, false, false, false, false, false, false, false, false, false, false, false,
            true, true, true, true, true, true, true
        ]));
        assert!(!act.none() && !act.all());
        // mixed

        act = BitVec::from_elem(31, false);
        act.set(3, true);
        act.set(17, true);
        act.set(30, true);
        assert!(act.eq_vec(&[
            false, false, false, true, false, false, false, false, false, false, false, false,
            false, false, false, false, false, true, false, false, false, false, false, false,
            false, false, false, false, false, false, true
        ]));
        assert!(!act.none() && !act.all());
    }

    #[test]
    fn test_32_elements() {
        // all 0

        let mut act = BitVec::from_elem(32, false);
        assert!(act.eq_vec(&[
            false, false, false, false, false, false, false, false, false, false, false, false,
            false, false, false, false, false, false, false, false, false, false, false, false,
            false, false, false, false, false, false, false, false
        ]));
        assert!(act.none() && !act.all());
        // all 1

        act = BitVec::from_elem(32, true);
        assert!(act.eq_vec(&[
            true, true, true, true, true, true, true, true, true, true, true, true, true, true,
            true, true, true, true, true, true, true, true, true, true, true, true, true, true,
            true, true, true, true
        ]));
        assert!(!act.none() && act.all());
        // mixed

        act = BitVec::from_elem(32, false);
        act.set(0, true);
        act.set(1, true);
        act.set(2, true);
        act.set(3, true);
        act.set(4, true);
        act.set(5, true);
        act.set(6, true);
        act.set(7, true);
        assert!(act.eq_vec(&[
            true, true, true, true, true, true, true, true, false, false, false, false, false,
            false, false, false, false, false, false, false, false, false, false, false, false,
            false, false, false, false, false, false, false
        ]));
        assert!(!act.none() && !act.all());
        // mixed

        act = BitVec::from_elem(32, false);
        act.set(16, true);
        act.set(17, true);
        act.set(18, true);
        act.set(19, true);
        act.set(20, true);
        act.set(21, true);
        act.set(22, true);
        act.set(23, true);
        assert!(act.eq_vec(&[
            false, false, false, false, false, false, false, false, false, false, false, false,
            false, false, false, false, true, true, true, true, true, true, true, true, false,
            false, false, false, false, false, false, false
        ]));
        assert!(!act.none() && !act.all());
        // mixed

        act = BitVec::from_elem(32, false);
        act.set(24, true);
        act.set(25, true);
        act.set(26, true);
        act.set(27, true);
        act.set(28, true);
        act.set(29, true);
        act.set(30, true);
        act.set(31, true);
        assert!(act.eq_vec(&[
            false, false, false, false, false, false, false, false, false, false, false, false,
            false, false, false, false, false, false, false, false, false, false, false, false,
            true, true, true, true, true, true, true, true
        ]));
        assert!(!act.none() && !act.all());
        // mixed

        act = BitVec::from_elem(32, false);
        act.set(3, true);
        act.set(17, true);
        act.set(30, true);
        act.set(31, true);
        assert!(act.eq_vec(&[
            false, false, false, true, false, false, false, false, false, false, false, false,
            false, false, false, false, false, true, false, false, false, false, false, false,
            false, false, false, false, false, false, true, true
        ]));
        assert!(!act.none() && !act.all());
    }

    #[test]
    fn test_33_elements() {
        // all 0

        let mut act = BitVec::from_elem(33, false);
        assert!(act.eq_vec(&[
            false, false, false, false, false, false, false, false, false, false, false, false,
            false, false, false, false, false, false, false, false, false, false, false, false,
            false, false, false, false, false, false, false, false, false
        ]));
        assert!(act.none() && !act.all());
        // all 1

        act = BitVec::from_elem(33, true);
        assert!(act.eq_vec(&[
            true, true, true, true, true, true, true, true, true, true, true, true, true, true,
            true, true, true, true, true, true, true, true, true, true, true, true, true, true,
            true, true, true, true, true
        ]));
        assert!(!act.none() && act.all());
        // mixed

        act = BitVec::from_elem(33, false);
        act.set(0, true);
        act.set(1, true);
        act.set(2, true);
        act.set(3, true);
        act.set(4, true);
        act.set(5, true);
        act.set(6, true);
        act.set(7, true);
        assert!(act.eq_vec(&[
            true, true, true, true, true, true, true, true, false, false, false, false, false,
            false, false, false, false, false, false, false, false, false, false, false, false,
            false, false, false, false, false, false, false, false
        ]));
        assert!(!act.none() && !act.all());
        // mixed

        act = BitVec::from_elem(33, false);
        act.set(16, true);
        act.set(17, true);
        act.set(18, true);
        act.set(19, true);
        act.set(20, true);
        act.set(21, true);
        act.set(22, true);
        act.set(23, true);
        assert!(act.eq_vec(&[
            false, false, false, false, false, false, false, false, false, false, false, false,
            false, false, false, false, true, true, true, true, true, true, true, true, false,
            false, false, false, false, false, false, false, false
        ]));
        assert!(!act.none() && !act.all());
        // mixed

        act = BitVec::from_elem(33, false);
        act.set(24, true);
        act.set(25, true);
        act.set(26, true);
        act.set(27, true);
        act.set(28, true);
        act.set(29, true);
        act.set(30, true);
        act.set(31, true);
        assert!(act.eq_vec(&[
            false, false, false, false, false, false, false, false, false, false, false, false,
            false, false, false, false, false, false, false, false, false, false, false, false,
            true, true, true, true, true, true, true, true, false
        ]));
        assert!(!act.none() && !act.all());
        // mixed

        act = BitVec::from_elem(33, false);
        act.set(3, true);
        act.set(17, true);
        act.set(30, true);
        act.set(31, true);
        act.set(32, true);
        assert!(act.eq_vec(&[
            false, false, false, true, false, false, false, false, false, false, false, false,
            false, false, false, false, false, true, false, false, false, false, false, false,
            false, false, false, false, false, false, true, true, true
        ]));
        assert!(!act.none() && !act.all());
    }

    #[test]
    fn test_equal_differing_sizes() {
        let v0 = BitVec::from_elem(10, false);
        let v1 = BitVec::from_elem(11, false);
        assert_ne!(v0, v1);
    }

    #[test]
    fn test_equal_greatly_differing_sizes() {
        let v0 = BitVec::from_elem(10, false);
        let v1 = BitVec::from_elem(110, false);
        assert_ne!(v0, v1);
    }

    #[test]
    fn test_equal_sneaky_small() {
        let mut a = BitVec::from_elem(1, false);
        a.set(0, true);

        let mut b = BitVec::from_elem(1, true);
        b.set(0, true);

        assert_eq!(a, b);
    }

    #[test]
    fn test_equal_sneaky_big() {
        let mut a = BitVec::from_elem(100, false);
        for i in 0..100 {
            a.set(i, true);
        }

        let mut b = BitVec::from_elem(100, true);
        for i in 0..100 {
            b.set(i, true);
        }

        assert_eq!(a, b);
    }

    #[test]
    fn test_from_bytes() {
        let bit_vec = BitVec::from_bytes(&[0b10110110, 0b00000000, 0b11111111]);
        let str = concat!("10110110", "00000000", "11111111");
        assert_eq!(format!("{}", bit_vec), str);
    }

    #[test]
    fn test_to_bytes() {
        let mut bv = BitVec::from_elem(3, true);
        bv.set(1, false);
        assert_eq!(bv.to_bytes(), [0b10100000]);

        let mut bv = BitVec::from_elem(9, false);
        bv.set(2, true);
        bv.set(8, true);
        assert_eq!(bv.to_bytes(), [0b00100000, 0b10000000]);
    }

    #[test]
    fn test_from_bools() {
        let bools = [true, false, true, true];
        let bit_vec: BitVec = bools.iter().copied().collect();
        assert_eq!(format!("{}", bit_vec), "1011");
    }

    #[test]
    fn test_to_bools() {
        let bools = vec![false, false, true, false, false, true, true, false];
        assert_eq!(
            BitVec::from_bytes(&[0b00100110])
                .iter()
                .collect::<Vec<bool>>(),
            bools
        );
    }

    #[test]
    fn test_bit_vec_iterator() {
        let bools = vec![true, false, true, true];
        let bit_vec: BitVec = bools.iter().copied().collect();

        assert_eq!(bit_vec.iter().collect::<Vec<bool>>(), bools);

        let long: Vec<_> = (0..10000).map(|i| i % 2 == 0).collect();
        let bit_vec: BitVec = long.iter().copied().collect();
        assert_eq!(bit_vec.iter().collect::<Vec<bool>>(), long)
    }

    #[test]
    fn test_small_difference() {
        let mut b1 = BitVec::from_elem(3, false);
        let mut b2 = BitVec::from_elem(3, false);
        b1.set(0, true);
        b1.set(1, true);
        b2.set(1, true);
        b2.set(2, true);
        assert!(b1.difference(&b2));
        assert!(b1[0]);
        assert!(!b1[1]);
        assert!(!b1[2]);
    }

    #[test]
    fn test_big_difference() {
        let mut b1 = BitVec::from_elem(100, false);
        let mut b2 = BitVec::from_elem(100, false);
        b1.set(0, true);
        b1.set(40, true);
        b2.set(40, true);
        b2.set(80, true);
        assert!(b1.difference(&b2));
        assert!(b1[0]);
        assert!(!b1[40]);
        assert!(!b1[80]);
    }

    #[test]
    fn test_small_xor() {
        let mut a = BitVec::from_bytes(&[0b0011]);
        let b = BitVec::from_bytes(&[0b0101]);
        let c = BitVec::from_bytes(&[0b0110]);
        assert!(a.xor(&b));
        assert_eq!(a, c);
    }

    #[test]
    fn test_small_xnor() {
        let mut a = BitVec::from_bytes(&[0b0011]);
        let b = BitVec::from_bytes(&[0b1111_0101]);
        let c = BitVec::from_bytes(&[0b1001]);
        assert!(a.xnor(&b));
        assert_eq!(a, c);
    }

    #[test]
    fn test_small_nand() {
        let mut a = BitVec::from_bytes(&[0b1111_0011]);
        let b = BitVec::from_bytes(&[0b1111_0101]);
        let c = BitVec::from_bytes(&[0b1110]);
        assert!(a.nand(&b));
        assert_eq!(a, c);
    }

    #[test]
    fn test_small_nor() {
        let mut a = BitVec::from_bytes(&[0b0011]);
        let b = BitVec::from_bytes(&[0b1111_0101]);
        let c = BitVec::from_bytes(&[0b1000]);
        assert!(a.nor(&b));
        assert_eq!(a, c);
    }

    #[test]
    fn test_big_xor() {
        let mut a = BitVec::from_bytes(&[
            // 88 bits
            0, 0, 0b00010100, 0, 0, 0, 0, 0b00110100, 0, 0, 0,
        ]);
        let b = BitVec::from_bytes(&[
            // 88 bits
            0, 0, 0b00010100, 0, 0, 0, 0, 0, 0, 0, 0b00110100,
        ]);
        let c = BitVec::from_bytes(&[
            // 88 bits
            0, 0, 0, 0, 0, 0, 0, 0b00110100, 0, 0, 0b00110100,
        ]);
        assert!(a.xor(&b));
        assert_eq!(a, c);
    }

    #[test]
    fn test_big_xnor() {
        let mut a = BitVec::from_bytes(&[
            // 88 bits
            0, 0, 0b00010100, 0, 0, 0, 0, 0b00110100, 0, 0, 0,
        ]);
        let b = BitVec::from_bytes(&[
            // 88 bits
            0, 0, 0b00010100, 0, 0, 0, 0, 0, 0, 0, 0b00110100,
        ]);
        let c = BitVec::from_bytes(&[
            // 88 bits
            !0,
            !0,
            !0,
            !0,
            !0,
            !0,
            !0,
            !0b00110100,
            !0,
            !0,
            !0b00110100,
        ]);
        assert!(a.xnor(&b));
        assert_eq!(a, c);
    }

    #[test]
    fn test_small_clear() {
        let mut b = BitVec::from_elem(14, true);
        assert!(!b.none() && b.all());
        b.clear();
        assert!(b.none() && !b.all());
    }

    #[test]
    fn test_big_clear() {
        let mut b = BitVec::from_elem(140, true);
        assert!(!b.none() && b.all());
        b.clear();
        assert!(b.none() && !b.all());
    }

    #[test]
    fn test_bit_vec_lt() {
        let mut a = BitVec::from_elem(5, false);
        let mut b = BitVec::from_elem(5, false);

        assert!(a >= b && b >= a);
        b.set(2, true);
        assert!(a < b);
        a.set(3, true);
        assert!(a < b);
        a.set(2, true);
        assert!(a >= b && b < a);
        b.set(0, true);
        assert!(a < b);
    }

    #[test]
    fn test_ord() {
        let mut a = BitVec::from_elem(5, false);
        let mut b = BitVec::from_elem(5, false);

        assert!(a == b);
        a.set(1, true);
        assert!(a > b && a >= b);
        assert!(b < a && b <= a);
        b.set(1, true);
        b.set(2, true);
        assert!(b > a && b >= a);
        assert!(a < b && a <= b);
    }

    #[test]
    fn test_small_bit_vec_tests() {
        let v = BitVec::from_bytes(&[0]);
        assert!(!v.all());
        assert!(!v.any());
        assert!(v.none());

        let v = BitVec::from_bytes(&[0b00010100]);
        assert!(!v.all());
        assert!(v.any());
        assert!(!v.none());

        let v = BitVec::from_bytes(&[0xFF]);
        assert!(v.all());
        assert!(v.any());
        assert!(!v.none());
    }

    #[test]
    fn test_big_bit_vec_tests() {
        let v = BitVec::from_bytes(&[
            // 88 bits
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ]);
        assert!(!v.all());
        assert!(!v.any());
        assert!(v.none());

        let v = BitVec::from_bytes(&[
            // 88 bits
            0, 0, 0b00010100, 0, 0, 0, 0, 0b00110100, 0, 0, 0,
        ]);
        assert!(!v.all());
        assert!(v.any());
        assert!(!v.none());

        let v = BitVec::from_bytes(&[
            // 88 bits
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
        ]);
        assert!(v.all());
        assert!(v.any());
        assert!(!v.none());
    }

    #[test]
    fn test_bit_vec_push_pop() {
        let mut s = BitVec::from_elem(5 * U32_BITS - 2, false);
        assert_eq!(s.len(), 5 * U32_BITS - 2);
        assert!(!s[5 * U32_BITS - 3]);
        s.push(true);
        s.push(true);
        assert!(s[5 * U32_BITS - 2]);
        assert!(s[5 * U32_BITS - 1]);
        // Here the internal vector will need to be extended
        s.push(false);
        assert!(!s[5 * U32_BITS]);
        s.push(false);
        assert!(!s[5 * U32_BITS + 1]);
        assert_eq!(s.len(), 5 * U32_BITS + 2);
        // Pop it all off
        assert_eq!(s.pop(), Some(false));
        assert_eq!(s.pop(), Some(false));
        assert_eq!(s.pop(), Some(true));
        assert_eq!(s.pop(), Some(true));
        assert_eq!(s.len(), 5 * U32_BITS - 2);
    }

    #[test]
    fn test_bit_vec_truncate() {
        let mut s = BitVec::from_elem(5 * U32_BITS, true);

        assert_eq!(s, BitVec::from_elem(5 * U32_BITS, true));
        assert_eq!(s.len(), 5 * U32_BITS);
        s.truncate(4 * U32_BITS);
        assert_eq!(s, BitVec::from_elem(4 * U32_BITS, true));
        assert_eq!(s.len(), 4 * U32_BITS);
        // Truncating to a size > s.len() should be a noop
        s.truncate(5 * U32_BITS);
        assert_eq!(s, BitVec::from_elem(4 * U32_BITS, true));
        assert_eq!(s.len(), 4 * U32_BITS);
        s.truncate(3 * U32_BITS - 10);
        assert_eq!(s, BitVec::from_elem(3 * U32_BITS - 10, true));
        assert_eq!(s.len(), 3 * U32_BITS - 10);
        s.truncate(0);
        assert_eq!(s, BitVec::from_elem(0, true));
        assert_eq!(s.len(), 0);
    }

    #[test]
    fn test_bit_vec_reserve() {
        let mut s = BitVec::from_elem(5 * U32_BITS, true);
        // Check capacity
        assert!(s.capacity() >= 5 * U32_BITS);
        s.reserve(2 * U32_BITS);
        assert!(s.capacity() >= 7 * U32_BITS);
        s.reserve(7 * U32_BITS);
        assert!(s.capacity() >= 12 * U32_BITS);
        s.reserve_exact(7 * U32_BITS);
        assert!(s.capacity() >= 12 * U32_BITS);
        s.reserve(7 * U32_BITS + 1);
        assert!(s.capacity() > 12 * U32_BITS);
        // Check that length hasn't changed
        assert_eq!(s.len(), 5 * U32_BITS);
        s.push(true);
        s.push(false);
        s.push(true);
        assert!(s[5 * U32_BITS - 1]);
        assert!(s[5 * U32_BITS]);
        assert!(!s[5 * U32_BITS + 1]);
        assert!(s[5 * U32_BITS + 2]);
    }

    #[test]
    fn test_bit_vec_grow() {
        let mut bit_vec = BitVec::from_bytes(&[0b10110110, 0b00000000, 0b10101010]);
        bit_vec.grow(32, true);
        assert_eq!(
            bit_vec,
            BitVec::from_bytes(&[0b10110110, 0b00000000, 0b10101010, 0xFF, 0xFF, 0xFF, 0xFF])
        );
        bit_vec.grow(64, false);
        assert_eq!(
            bit_vec,
            BitVec::from_bytes(&[
                0b10110110, 0b00000000, 0b10101010, 0xFF, 0xFF, 0xFF, 0xFF, 0, 0, 0, 0, 0, 0, 0, 0
            ])
        );
        bit_vec.grow(16, true);
        assert_eq!(
            bit_vec,
            BitVec::from_bytes(&[
                0b10110110, 0b00000000, 0b10101010, 0xFF, 0xFF, 0xFF, 0xFF, 0, 0, 0, 0, 0, 0, 0, 0,
                0xFF, 0xFF
            ])
        );
    }

    #[test]
    fn test_bit_vec_extend() {
        let mut bit_vec = BitVec::from_bytes(&[0b10110110, 0b00000000, 0b11111111]);
        let ext = BitVec::from_bytes(&[0b01001001, 0b10010010, 0b10111101]);
        bit_vec.extend(ext.iter());
        assert_eq!(
            bit_vec,
            BitVec::from_bytes(&[
                0b10110110, 0b00000000, 0b11111111, 0b01001001, 0b10010010, 0b10111101
            ])
        );
    }

    #[test]
    fn test_bit_vec_append() {
        // Append to BitVec that holds a multiple of U32_BITS bits
        let mut a = BitVec::from_bytes(&[0b10100000, 0b00010010, 0b10010010, 0b00110011]);
        let mut b = BitVec::new();
        b.push(false);
        b.push(true);
        b.push(true);

        a.append(&mut b);

        assert_eq!(a.len(), 35);
        assert_eq!(b.len(), 0);
        assert!(b.capacity() >= 3);

        assert!(a.eq_vec(&[
            true, false, true, false, false, false, false, false, false, false, false, true, false,
            false, true, false, true, false, false, true, false, false, true, false, false, false,
            true, true, false, false, true, true, false, true, true
        ]));

        // Append to arbitrary BitVec
        let mut a = BitVec::new();
        a.push(true);
        a.push(false);

        let mut b =
            BitVec::from_bytes(&[0b10100000, 0b00010010, 0b10010010, 0b00110011, 0b10010101]);

        a.append(&mut b);

        assert_eq!(a.len(), 42);
        assert_eq!(b.len(), 0);
        assert!(b.capacity() >= 40);

        assert!(a.eq_vec(&[
            true, false, true, false, true, false, false, false, false, false, false, false, false,
            true, false, false, true, false, true, false, false, true, false, false, true, false,
            false, false, true, true, false, false, true, true, true, false, false, true, false,
            true, false, true
        ]));

        // Append to empty BitVec
        let mut a = BitVec::new();
        let mut b =
            BitVec::from_bytes(&[0b10100000, 0b00010010, 0b10010010, 0b00110011, 0b10010101]);

        a.append(&mut b);

        assert_eq!(a.len(), 40);
        assert_eq!(b.len(), 0);
        assert!(b.capacity() >= 40);

        assert!(a.eq_vec(&[
            true, false, true, false, false, false, false, false, false, false, false, true, false,
            false, true, false, true, false, false, true, false, false, true, false, false, false,
            true, true, false, false, true, true, true, false, false, true, false, true, false,
            true
        ]));

        // Append empty BitVec
        let mut a =
            BitVec::from_bytes(&[0b10100000, 0b00010010, 0b10010010, 0b00110011, 0b10010101]);
        let mut b = BitVec::new();

        a.append(&mut b);

        assert_eq!(a.len(), 40);
        assert_eq!(b.len(), 0);

        assert!(a.eq_vec(&[
            true, false, true, false, false, false, false, false, false, false, false, true, false,
            false, true, false, true, false, false, true, false, false, true, false, false, false,
            true, true, false, false, true, true, true, false, false, true, false, true, false,
            true
        ]));
    }

    #[test]
    fn test_bit_vec_split_off() {
        // Split at 0
        let mut a = BitVec::new();
        a.push(true);
        a.push(false);
        a.push(false);
        a.push(true);

        let b = a.split_off(0);

        assert_eq!(a.len(), 0);
        assert_eq!(b.len(), 4);

        assert!(b.eq_vec(&[true, false, false, true]));

        // Split at last bit
        a.truncate(0);
        a.push(true);
        a.push(false);
        a.push(false);
        a.push(true);

        let b = a.split_off(4);

        assert_eq!(a.len(), 4);
        assert_eq!(b.len(), 0);

        assert!(a.eq_vec(&[true, false, false, true]));

        // Split at block boundary
        let mut a =
            BitVec::from_bytes(&[0b10100000, 0b00010010, 0b10010010, 0b00110011, 0b11110011]);

        let b = a.split_off(32);

        assert_eq!(a.len(), 32);
        assert_eq!(b.len(), 8);

        assert!(a.eq_vec(&[
            true, false, true, false, false, false, false, false, false, false, false, true, false,
            false, true, false, true, false, false, true, false, false, true, false, false, false,
            true, true, false, false, true, true
        ]));
        assert!(b.eq_vec(&[true, true, true, true, false, false, true, true]));

        // Don't split at block boundary
        let mut a = BitVec::from_bytes(&[
            0b10100000, 0b00010010, 0b10010010, 0b00110011, 0b01101011, 0b10101101,
        ]);

        let b = a.split_off(13);

        assert_eq!(a.len(), 13);
        assert_eq!(b.len(), 35);

        assert!(a.eq_vec(&[
            true, false, true, false, false, false, false, false, false, false, false, true, false
        ]));
        assert!(b.eq_vec(&[
            false, true, false, true, false, false, true, false, false, true, false, false, false,
            true, true, false, false, true, true, false, true, true, false, true, false, true,
            true, true, false, true, false, true, true, false, true
        ]));
    }

    #[test]
    fn test_into_iter() {
        let bools = [true, false, true, true];
        let bit_vec: BitVec = bools.iter().copied().collect();
        let mut iter = bit_vec.into_iter();
        assert_eq!(Some(true), iter.next());
        assert_eq!(Some(false), iter.next());
        assert_eq!(Some(true), iter.next());
        assert_eq!(Some(true), iter.next());
        assert_eq!(None, iter.next());
        assert_eq!(None, iter.next());

        let bit_vec: BitVec = bools.iter().copied().collect();
        let mut iter = bit_vec.into_iter();
        assert_eq!(Some(true), iter.next_back());
        assert_eq!(Some(true), iter.next_back());
        assert_eq!(Some(false), iter.next_back());
        assert_eq!(Some(true), iter.next_back());
        assert_eq!(None, iter.next_back());
        assert_eq!(None, iter.next_back());

        let bit_vec: BitVec = bools.iter().copied().collect();
        let mut iter = bit_vec.into_iter();
        assert_eq!(Some(true), iter.next_back());
        assert_eq!(Some(true), iter.next());
        assert_eq!(Some(false), iter.next());
        assert_eq!(Some(true), iter.next_back());
        assert_eq!(None, iter.next());
        assert_eq!(None, iter.next_back());
    }

    #[test]
    fn iter() {
        let b = BitVec::with_capacity(10);
        let _a: Iter = b.iter();
    }

    #[cfg(feature = "serialization")]
    #[test]
    fn test_serialization() {
        let bit_vec: BitVec = BitVec::new();
        let serialized = json_codec::to_string(&bit_vec).unwrap();
        let unserialized: BitVec = json_codec::from_str(&serialized).unwrap();
        assert_eq!(bit_vec, unserialized);

        let bools = vec![true, false, true, true];
        let bit_vec: BitVec = bools.iter().map(|n| *n).collect();
        let serialized = json_codec::to_string(&bit_vec).unwrap();
        let unserialized = json_codec::from_str(&serialized).unwrap();
        assert_eq!(bit_vec, unserialized);
    }







    #[test]
    fn test_bit_vec_unaligned_small_append() {
        let mut a = BitVec::from_elem(8, false);
        a.set(7, true);

        let mut b = BitVec::from_elem(16, false);
        b.set(14, true);

        let mut c = BitVec::from_elem(8, false);
        c.set(6, true);
        c.set(7, true);

        a.append(&mut b);
        a.append(&mut c);

        assert_eq!(&[1, 0, 2, 3][..], &*a.to_bytes());
    }

    #[test]
    fn test_bit_vec_unaligned_large_append() {
        let mut a = BitVec::from_elem(48, false);
        a.set(47, true);

        let mut b = BitVec::from_elem(48, false);
        b.set(46, true);

        let mut c = BitVec::from_elem(48, false);
        c.set(46, true);
        c.set(47, true);

        a.append(&mut b);
        a.append(&mut c);

        assert_eq!(
            &[
                0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x03
            ][..],
            &*a.to_bytes()
        );
    }

    #[test]
    fn test_bit_vec_append_aligned_to_unaligned() {
        let mut a = BitVec::from_elem(2, true);
        let mut b = BitVec::from_elem(32, false);
        let mut c = BitVec::from_elem(8, true);
        a.append(&mut b);
        a.append(&mut c);
        assert_eq!(&[0xc0, 0x00, 0x00, 0x00, 0x3f, 0xc0][..], &*a.to_bytes());
    }

    #[test]
    fn test_count_ones() {
        for i in 0..1000 {
            let mut t = BitVec::from_elem(i, true);
            let mut f = BitVec::from_elem(i, false);
            assert_eq!(i as u64, t.count_ones());
            assert_eq!(0_u64, f.count_ones());
            if i > 20 {
                t.set(10, false);
                t.set(i - 10, false);
                assert_eq!(i - 2, t.count_ones() as usize);
                f.set(10, true);
                f.set(i - 10, true);
                assert_eq!(2, f.count_ones());
            }
        }
    }

    #[test]
    fn test_count_zeros() {
        for i in 0..1000 {
            let mut tbits = BitVec::from_elem(i, true);
            let mut fbits = BitVec::from_elem(i, false);
            assert_eq!(i as u64, fbits.count_zeros());
            assert_eq!(0_u64, tbits.count_zeros());
            if i > 20 {
                fbits.set(10, true);
                fbits.set(i - 10, true);
                assert_eq!(i - 2, fbits.count_zeros() as usize);
                tbits.set(10, false);
                tbits.set(i - 10, false);
                assert_eq!(2, tbits.count_zeros());
            }
        }
    }

    #[test]
    fn test_get_mut() {
        let mut a = BitVec::from_elem(3, false);
        let mut a_bit_1 = a.get_mut(1).unwrap();
        assert!(!*a_bit_1);
        *a_bit_1 = true;
        drop(a_bit_1);
        assert!(a.eq_vec(&[false, true, false]));
    }
    #[test]
    fn test_iter_mut() {
        let mut a = BitVec::from_elem(8, false);
        a.iter_mut().enumerate().for_each(|(index, mut bit)| {
            *bit = index % 2 == 1;
        });
        assert!(a.eq_vec(&[false, true, false, true, false, true, false, true]));
    }

    #[test]
    fn test_insert_at_zero() {
        let mut v = BitVec::new();

        v.insert(0, false);
        v.insert(0, true);
        v.insert(0, false);
        v.insert(0, true);
        v.insert(0, false);
        v.insert(0, true);

        assert_eq!(v.len(), 6);
        assert_eq!(v.storage().len(), 1);
        assert!(v.eq_vec(&[true, false, true, false, true, false]));
    }

    #[test]
    fn test_insert_at_end() {
        let mut v = BitVec::new();

        v.insert(v.len(), true);
        v.insert(v.len(), false);
        v.insert(v.len(), true);
        v.insert(v.len(), false);
        v.insert(v.len(), true);
        v.insert(v.len(), false);

        assert_eq!(v.storage().len(), 1);
        assert_eq!(v.len(), 6);
        assert!(v.eq_vec(&[true, false, true, false, true, false]));
    }

    #[test]
    fn test_insert_at_block_boundaries() {
        let mut v = BitVec::from_elem(32, false);

        assert_eq!(v.storage().len(), 1);

        v.insert(31, true);

        assert_eq!(v.len(), 33);

        assert!(matches!(v.get(31), Some(true)));
        assert!(v.eq_vec(&[
            false, false, false, false, false, false, false, false, false, false, false, false,
            false, false, false, false, false, false, false, false, false, false, false, false,
            false, false, false, false, false, false, false, true, false
        ]));

        assert_eq!(v.storage().len(), 2);
    }

    #[test]
    fn test_insert_at_block_boundaries_1() {
        let mut v = BitVec::from_elem(64, false);

        assert_eq!(v.storage().len(), 2);

        v.insert(63, true);

        assert_eq!(v.len(), 65);

        assert!(matches!(v.get(63), Some(true)));
        assert!(v.eq_vec(&[
            false, false, false, false, false, false, false, false, false, false, false, false,
            false, false, false, false, false, false, false, false, false, false, false, false,
            false, false, false, false, false, false, false, false, false, false, false, false,
            false, false, false, false, false, false, false, false, false, false, false, false,
            false, false, false, false, false, false, false, false, false, false, false, false,
            false, false, false, true, false
        ]));

        assert_eq!(v.storage().len(), 3);
    }
}
