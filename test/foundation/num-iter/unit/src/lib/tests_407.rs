#[cfg(test)]
mod tests {
    use core::cmp::Ordering;
    use core::iter;
    use core::ops::{Add, Mul};

    use num_traits::{One, ToPrimitive};

    #[test]
    fn test_range() {
        /// A mock type to check Range when ToPrimitive returns None
        struct Foo;

        impl ToPrimitive for Foo {
            fn to_i64(&self) -> Option<i64> {
                None
            }
            fn to_u64(&self) -> Option<u64> {
                None
            }
        }

        impl Add<Foo> for Foo {
            type Output = Foo;

            fn add(self, _: Foo) -> Foo {
                Foo
            }
        }

        impl PartialEq for Foo {
            fn eq(&self, _: &Foo) -> bool {
                true
            }
        }

        impl PartialOrd for Foo {
            fn partial_cmp(&self, _: &Foo) -> Option<Ordering> {
                None
            }
        }

        impl Clone for Foo {
            fn clone(&self) -> Foo {
                Foo
            }
        }

        impl Mul<Foo> for Foo {
            type Output = Foo;

            fn mul(self, _: Foo) -> Foo {
                Foo
            }
        }

        impl One for Foo {
            fn one() -> Foo {
                Foo
            }
        }

        assert_eq!(super::range(Foo, Foo).size_hint(), (0, None));
        assert!(super::range(0, 5).eq([0, 1, 2, 3, 4].iter().cloned()));
        assert!(super::range(-10, -1).eq([-10, -9, -8, -7, -6, -5, -4, -3, -2].iter().cloned()));
        assert!(super::range(0, 5).rev().eq([4, 3, 2, 1, 0].iter().cloned()));
        assert_eq!(super::range(200, -5).count(), 0);
        assert_eq!(super::range(200, -5).rev().count(), 0);
        assert_eq!(super::range(200, 200).count(), 0);
        assert_eq!(super::range(200, 200).rev().count(), 0);

        assert_eq!(super::range(0, 100).size_hint(), (100, Some(100)));
        // this test is only meaningful when sizeof usize < sizeof u64
        assert_eq!(
            super::range(usize::MAX - 1, usize::MAX).size_hint(),
            (1, Some(1))
        );
        assert_eq!(super::range(-10, -1).size_hint(), (9, Some(9)));
        assert_eq!(
            super::range(isize::MIN, isize::MAX).size_hint(),
            (usize::MAX, Some(usize::MAX))
        );
    }

    #[test]
    fn test_range_128() {
        assert!(super::range(0i128, 5).eq([0, 1, 2, 3, 4].iter().cloned()));
        assert!(super::range(-10i128, -1).eq([-10, -9, -8, -7, -6, -5, -4, -3, -2].iter().cloned()));
        assert!(super::range(0u128, 5)
            .rev()
            .eq([4, 3, 2, 1, 0].iter().cloned()));

        assert_eq!(
            super::range(i128::MIN, i128::MIN + 1).size_hint(),
            (1, Some(1))
        );
        assert_eq!(
            super::range(i128::MAX - 1, i128::MAX).size_hint(),
            (1, Some(1))
        );
        assert_eq!(
            super::range(i128::MIN, i128::MAX).size_hint(),
            (usize::MAX, None)
        );

        assert_eq!(
            super::range(u128::MAX - 1, u128::MAX).size_hint(),
            (1, Some(1))
        );
        assert_eq!(
            super::range(0, usize::MAX as u128).size_hint(),
            (usize::MAX, Some(usize::MAX))
        );
        assert_eq!(
            super::range(0, usize::MAX as u128 + 1).size_hint(),
            (usize::MAX, None)
        );
        assert_eq!(super::range(0, i128::MAX).size_hint(), (usize::MAX, None));
    }

    #[test]
    fn test_range_inclusive() {
        assert!(super::range_inclusive(0, 5).eq([0, 1, 2, 3, 4, 5].iter().cloned()));
        assert!(super::range_inclusive(0, 5)
            .rev()
            .eq([5, 4, 3, 2, 1, 0].iter().cloned()));
        assert_eq!(super::range_inclusive(200, -5).count(), 0);
        assert_eq!(super::range_inclusive(200, -5).rev().count(), 0);
        assert!(super::range_inclusive(200, 200).eq(iter::once(200)));
        assert!(super::range_inclusive(200, 200).rev().eq(iter::once(200)));
        assert_eq!(
            super::range_inclusive(isize::MIN, isize::MAX - 1).size_hint(),
            (usize::MAX, Some(usize::MAX))
        );
        assert_eq!(
            super::range_inclusive(isize::MIN, isize::MAX).size_hint(),
            (usize::MAX, None)
        );
    }

    #[test]
    fn test_range_inclusive_128() {
        assert!(super::range_inclusive(0u128, 5).eq([0, 1, 2, 3, 4, 5].iter().cloned()));
        assert!(super::range_inclusive(0u128, 5)
            .rev()
            .eq([5, 4, 3, 2, 1, 0].iter().cloned()));
        assert_eq!(super::range_inclusive(200i128, -5).count(), 0);
        assert_eq!(super::range_inclusive(200i128, -5).rev().count(), 0);
        assert!(super::range_inclusive(200u128, 200).eq(iter::once(200)));
        assert!(super::range_inclusive(200u128, 200)
            .rev()
            .eq(iter::once(200)));
        assert_eq!(
            super::range_inclusive(isize::MIN as i128, isize::MAX as i128 - 1).size_hint(),
            (usize::MAX, Some(usize::MAX))
        );
        assert_eq!(
            super::range_inclusive(isize::MIN as i128, isize::MAX as i128).size_hint(),
            (usize::MAX, None)
        );
        assert_eq!(
            super::range_inclusive(isize::MIN as i128, isize::MAX as i128 + 1).size_hint(),
            (usize::MAX, None)
        );
        assert_eq!(
            super::range_inclusive(i128::MIN, i128::MAX).size_hint(),
            (usize::MAX, None)
        );
    }

    #[test]
    fn test_range_step() {
        assert!(super::range_step(0, 20, 5).eq([0, 5, 10, 15].iter().cloned()));
        assert!(super::range_step(20, 0, -5).eq([20, 15, 10, 5].iter().cloned()));
        assert!(super::range_step(20, 0, -6).eq([20, 14, 8, 2].iter().cloned()));
        assert!(super::range_step(200u8, 255, 50).eq([200u8, 250].iter().cloned()));
        assert!(super::range_step(200, -5, 1).eq(iter::empty()));
        assert!(super::range_step(200, 200, 1).eq(iter::empty()));
    }

    #[test]
    fn test_range_step_128() {
        const UMAX: u128 = u128::MAX;

        assert!(super::range_step(0u128, 20, 5).eq([0, 5, 10, 15].iter().cloned()));
        assert!(super::range_step(20i128, 0, -5).eq([20, 15, 10, 5].iter().cloned()));
        assert!(super::range_step(20i128, 0, -6).eq([20, 14, 8, 2].iter().cloned()));
        assert!(super::range_step(UMAX - 55, UMAX, 50).eq([UMAX - 55, UMAX - 5].iter().cloned()));
        assert!(super::range_step(200i128, -5, 1).eq(iter::empty()));
        assert!(super::range_step(200i128, 200, 1).eq(iter::empty()));
    }

    #[test]
    fn test_range_step_inclusive() {
        assert!(super::range_step_inclusive(0, 20, 5).eq([0, 5, 10, 15, 20].iter().cloned()));
        assert!(super::range_step_inclusive(20, 0, -5).eq([20, 15, 10, 5, 0].iter().cloned()));
        assert!(super::range_step_inclusive(20, 0, -6).eq([20, 14, 8, 2].iter().cloned()));
        assert!(super::range_step_inclusive(200u8, 255, 50).eq([200u8, 250].iter().cloned()));
        assert!(super::range_step_inclusive(200, -5, 1).eq(iter::empty()));
        assert!(super::range_step_inclusive(200, 200, 1).eq(iter::once(200)));
    }

    #[test]
    fn test_range_step_inclusive_128() {
        const UMAX: u128 = u128::MAX;

        assert!(super::range_step_inclusive(0u128, 20, 5).eq([0, 5, 10, 15, 20].iter().cloned()));
        assert!(super::range_step_inclusive(20i128, 0, -5).eq([20, 15, 10, 5, 0].iter().cloned()));
        assert!(super::range_step_inclusive(20i128, 0, -6).eq([20, 14, 8, 2].iter().cloned()));
        assert!(super::range_step_inclusive(UMAX - 55, UMAX, 50)
            .eq([UMAX - 55, UMAX - 5].iter().cloned()));
        assert!(super::range_step_inclusive(200i128, -5, 1).eq(iter::empty()));
        assert!(super::range_step_inclusive(200i128, 200, 1).eq(iter::once(200)));
    }

    #[test]
    fn test_range_from() {
        assert!(super::range_from(10u8)
            .take(5)
            .eq([10, 11, 12, 13, 14].iter().cloned()));
        assert_eq!(super::range_from(10u8).size_hint(), (usize::MAX, None));
    }

    #[test]
    fn test_range_step_from() {
        assert!(super::range_step_from(10u8, 2u8)
            .take(5)
            .eq([10, 12, 14, 16, 18].iter().cloned()));
        assert_eq!(
            super::range_step_from(10u8, 2u8).size_hint(),
            (usize::MAX, None)
        );

        assert!(super::range_step_from(10u8, 1u8)
            .take(5)
            .eq([10, 11, 12, 13, 14].iter().cloned()));
        assert_eq!(
            super::range_step_from(10u8, 1u8).size_hint(),
            (usize::MAX, None)
        );

        assert!(super::range_step_from(10u8, 0u8)
            .take(5)
            .eq([10, 10, 10, 10, 10].iter().cloned()));
        assert_eq!(
            super::range_step_from(10u8, 0u8).size_hint(),
            (usize::MAX, None)
        );

        assert!(super::range_step_from(10i8, 2i8)
            .take(5)
            .eq([10, 12, 14, 16, 18].iter().cloned()));
        assert_eq!(
            super::range_step_from(10i8, 2i8).size_hint(),
            (usize::MAX, None)
        );

        assert!(super::range_step_from(10i8, 1i8)
            .take(5)
            .eq([10, 11, 12, 13, 14].iter().cloned()));
        assert_eq!(
            super::range_step_from(10i8, 1i8).size_hint(),
            (usize::MAX, None)
        );

        assert!(super::range_step_from(10i8, 0i8)
            .take(5)
            .eq([10, 10, 10, 10, 10].iter().cloned()));
        assert_eq!(
            super::range_step_from(10i8, 0i8).size_hint(),
            (usize::MAX, None)
        );

        assert!(super::range_step_from(10i8, -1i8)
            .take(5)
            .eq([10, 9, 8, 7, 6].iter().cloned()));
        assert_eq!(
            super::range_step_from(10i8, -1i8).size_hint(),
            (usize::MAX, None)
        );

        assert!(super::range_step_from(10i8, -2i8)
            .take(5)
            .eq([10, 8, 6, 4, 2].iter().cloned()));
        assert_eq!(
            super::range_step_from(10i8, -2i8).size_hint(),
            (usize::MAX, None)
        );
    }
}
