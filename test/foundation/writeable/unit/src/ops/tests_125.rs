#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add() {
        assert_eq!(LengthHint::exact(3) + 2, LengthHint::exact(5));
        assert_eq!(
            LengthHint::exact(3) + LengthHint::exact(2),
            LengthHint::exact(5)
        );
        assert_eq!(
            LengthHint::exact(3) + LengthHint::undefined(),
            LengthHint::at_least(3)
        );

        assert_eq!(LengthHint::undefined() + 2, LengthHint::at_least(2));
        assert_eq!(
            LengthHint::undefined() + LengthHint::exact(2),
            LengthHint::at_least(2)
        );
        assert_eq!(
            LengthHint::undefined() + LengthHint::undefined(),
            LengthHint::undefined()
        );

        assert_eq!(
            LengthHint::at_least(15) + LengthHint::exact(3),
            LengthHint::at_least(18)
        );

        assert_eq!(
            LengthHint::at_least(15) + LengthHint::at_most(3),
            LengthHint::at_least(15)
        );

        assert_eq!(LengthHint::between(48, 92) + 5, LengthHint::between(53, 97));

        let mut len = LengthHint::exact(5);
        len += LengthHint::exact(3);
        assert_eq!(len, LengthHint::exact(8));
        len += 2;
        assert_eq!(len, LengthHint::exact(10));
        len += LengthHint::undefined();
        assert_eq!(len, LengthHint::at_least(10));

        len += LengthHint::exact(3);
        assert_eq!(len, LengthHint::at_least(13));
        len += 2;
        assert_eq!(len, LengthHint::at_least(15));
        len += LengthHint::undefined();
        assert_eq!(len, LengthHint::at_least(15));

        assert_eq!(
            LengthHint::between(usize::MAX - 10, usize::MAX - 5) + LengthHint::exact(20),
            LengthHint::at_least(usize::MAX)
        );
    }

    #[test]
    fn test_sum() {
        let lens = [
            LengthHint::exact(4),
            LengthHint::exact(1),
            LengthHint::exact(1),
        ];
        assert_eq!(
            lens.iter().copied().sum::<LengthHint>(),
            LengthHint::exact(6)
        );

        let lens = [
            LengthHint::exact(4),
            LengthHint::undefined(),
            LengthHint::at_least(1),
        ];
        assert_eq!(
            lens.iter().copied().sum::<LengthHint>(),
            LengthHint::at_least(5)
        );

        let lens = [
            LengthHint::exact(4),
            LengthHint::undefined(),
            LengthHint::at_most(1),
        ];
        assert_eq!(
            lens.iter().copied().sum::<LengthHint>(),
            LengthHint::at_least(4)
        );

        let lens = [4, 1, 1];
        assert_eq!(
            lens.iter().copied().sum::<LengthHint>(),
            LengthHint::exact(6)
        );
    }

    #[test]
    fn test_mul() {
        assert_eq!(LengthHint::exact(3) * 2, LengthHint::exact(6));

        assert_eq!(LengthHint::undefined() * 2, LengthHint::undefined());

        assert_eq!(
            LengthHint::between(48, 92) * 2,
            LengthHint::between(96, 184)
        );

        let mut len = LengthHint::exact(5);
        len *= 2;
        assert_eq!(len, LengthHint::exact(10));

        assert_eq!(
            LengthHint::between(usize::MAX - 10, usize::MAX - 5) * 2,
            LengthHint::at_least(usize::MAX)
        );
    }

    #[test]
    fn test_bitor() {
        assert_eq!(
            LengthHint::exact(3) | LengthHint::exact(2),
            LengthHint::between(2, 3)
        );
        assert_eq!(
            LengthHint::exact(3) | LengthHint::undefined(),
            LengthHint::undefined()
        );

        assert_eq!(
            LengthHint::undefined() | LengthHint::undefined(),
            LengthHint::undefined()
        );

        assert_eq!(
            LengthHint::exact(10) | LengthHint::exact(10),
            LengthHint::exact(10)
        );

        assert_eq!(
            LengthHint::at_least(15) | LengthHint::exact(3),
            LengthHint::at_least(3)
        );

        assert_eq!(
            LengthHint::at_least(15) | LengthHint::at_most(18),
            LengthHint::undefined()
        );

        assert_eq!(
            LengthHint::at_least(15) | LengthHint::at_least(18),
            LengthHint::at_least(15)
        );

        assert_eq!(
            LengthHint::at_most(15) | LengthHint::at_most(18),
            LengthHint::at_most(18)
        );

        assert_eq!(
            LengthHint::between(5, 10) | LengthHint::at_most(3),
            LengthHint::at_most(10)
        );

        let mut len = LengthHint::exact(5);
        len |= LengthHint::exact(3);
        assert_eq!(len, LengthHint::between(5, 3));
    }
}
