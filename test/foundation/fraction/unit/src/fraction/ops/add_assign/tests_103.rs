#[cfg(test)]
mod tests {
    use super::GenericFraction;
    use crate::{One, Zero};

    type F = GenericFraction<u8>;

    #[test]
    fn owned() {
        {
            let mut v = F::zero();
            v += F::infinity();
            assert_eq!(v, F::infinity());
        }

        {
            let mut v = F::infinity();
            v += F::nan();
            assert_eq!(v, F::nan());
        }

        {
            let mut v = F::infinity();
            v += F::infinity();
            assert_eq!(v, F::infinity());
        }

        {
            let mut v = F::infinity();
            v += F::neg_infinity();
            assert_eq!(v, F::nan());
        }

        {
            let mut v = F::infinity();
            v += F::one();
            assert_eq!(v, F::infinity());
        }

        {
            let mut v = F::one();
            v += F::new_neg(1, 1);
            assert_eq!(v, F::zero());
        }

        {
            let mut v = F::one();
            v += F::new_neg(2, 1);
            assert_eq!(v, F::new_neg(1, 1));
        }

        {
            let mut v = F::new_neg(1, 1);
            v += F::new_neg(1, 1);
            assert_eq!(v, F::new_neg(2, 1));
        }

        {
            let mut v = F::new_neg(1, 1);
            v += F::new(1, 1);
            assert_eq!(v, F::zero());
        }

        {
            let mut v = F::new_neg(2, 1);
            v += F::new(1, 1);
            assert_eq!(v, F::new_neg(1, 1));
        }
    }

    #[test]
    fn refs() {
        {
            let mut v = F::zero();
            v += &F::infinity();
            assert_eq!(v, F::infinity());
        }

        {
            let mut v = F::infinity();
            v += &F::nan();
            assert_eq!(v, F::nan());
        }

        {
            let mut v = F::infinity();
            v += &F::infinity();
            assert_eq!(v, F::infinity());
        }

        {
            let mut v = F::infinity();
            v += &F::neg_infinity();
            assert_eq!(v, F::nan());
        }

        {
            let mut v = F::infinity();
            v += &F::one();
            assert_eq!(v, F::infinity());
        }

        {
            let mut v = F::one();
            v += &F::new_neg(1, 1);
            assert_eq!(v, F::zero());
        }

        {
            let mut v = F::one();
            v += &F::new_neg(2, 1);
            assert_eq!(v, F::new_neg(1, 1));
        }

        {
            let mut v = F::new_neg(1, 1);
            v += &F::new_neg(1, 1);
            assert_eq!(v, F::new_neg(2, 1));
        }

        {
            let mut v = F::new_neg(1, 1);
            v += &F::new(1, 1);
            assert_eq!(v, F::zero());
        }

        {
            let mut v = F::new_neg(2, 1);
            v += &F::new(1, 1);
            assert_eq!(v, F::new_neg(1, 1));
        }
    }

    #[test]
    fn cast() {
        {
            let mut v = F::zero();
            v += 1;
            assert_eq!(v, F::one());
        }

        {
            let mut v = F::zero();
            v += 1;
            assert_eq!(v, F::one());
        }
    }
}
