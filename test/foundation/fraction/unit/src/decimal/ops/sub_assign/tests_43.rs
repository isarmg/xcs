#[cfg(test)]
mod tests {
    use crate::{GenericDecimal, One, Zero};

    type F = GenericDecimal<u8, u8>;

    #[test]
    fn owned() {
        {
            let mut v = F::nan();
            v -= F::nan();
            assert_eq!(v, F::nan());
        }

        {
            let mut v = F::infinity();
            v -= F::nan();
            assert_eq!(v, F::nan());
        }

        {
            let mut v = F::infinity();
            v -= F::infinity();
            assert_eq!(v, F::nan());
        }

        {
            let mut v = F::infinity();
            v -= F::neg_infinity();
            assert_eq!(v, F::infinity());
        }

        {
            let mut v = F::infinity();
            v -= F::one();
            assert_eq!(v, F::infinity());
        }

        {
            let mut v = F::one();
            v -= F::nan();
            assert_eq!(v, F::nan());
        }

        {
            let mut v = F::one();
            v -= F::infinity();
            assert_eq!(v, F::neg_infinity());
        }

        {
            let mut v = F::one();
            v -= F::neg_infinity();
            assert_eq!(v, F::infinity());
        }

        {
            let mut v = F::one();
            v -= -F::one();
            assert_eq!(v, F::from(2));
        }

        {
            let mut v = -F::one();
            v -= F::from(2);
            assert_eq!(v, -F::from(3));
        }

        {
            let mut v = -F::one();
            v -= -F::one();
            assert_eq!(v, F::zero());
        }

        {
            let mut v = -F::one();
            v -= -F::from(2);
            assert_eq!(v, F::one());
        }
    }

    #[test]
    fn refs() {
        {
            let mut v = F::nan();
            v -= &F::nan();
            assert_eq!(v, F::nan());
        }

        {
            let mut v = F::infinity();
            v -= &F::nan();
            assert_eq!(v, F::nan());
        }

        {
            let mut v = F::infinity();
            v -= &F::infinity();
            assert_eq!(v, F::nan());
        }

        {
            let mut v = F::infinity();
            v -= &F::neg_infinity();
            assert_eq!(v, F::infinity());
        }

        {
            let mut v = F::infinity();
            v -= &F::one();
            assert_eq!(v, F::infinity());
        }

        {
            let mut v = F::one();
            v -= &F::nan();
            assert_eq!(v, F::nan());
        }

        {
            let mut v = F::one();
            v -= &F::infinity();
            assert_eq!(v, F::neg_infinity());
        }

        {
            let mut v = F::one();
            v -= &F::neg_infinity();
            assert_eq!(v, F::infinity());
        }

        {
            let mut v = F::one();
            v -= &(-F::one());
            assert_eq!(v, F::from(2));
        }

        {
            let mut v = -F::one();
            v -= &F::from(2);
            assert_eq!(v, -F::from(3));
        }

        {
            let mut v = -F::one();
            v -= &(-F::one());
            assert_eq!(v, F::zero());
        }

        {
            let mut v = -F::one();
            v -= &(-F::from(2));
            assert_eq!(v, F::one());
        }
    }

    #[test]
    fn cast() {
        {
            let mut v = F::one();
            v -= 1;
            assert_eq!(v, F::zero());
        }

        {
            let mut v = F::one();
            v -= 1;
            assert_eq!(v, F::zero());
        }
    }
}
