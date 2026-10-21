#[cfg(test)]
mod bit_creation_tests {
    #[test]
    fn bit_creation() {
        {
            use crate::{B0, B1};
            let _: B0 = B0::new();
            let _: B1 = B1::new();
        }

        {
            use crate::{Bit, B0, B1};

            let _: B0 = <B0 as Bit>::new();
            let _: B1 = <B1 as Bit>::new();
        }
    }
}
