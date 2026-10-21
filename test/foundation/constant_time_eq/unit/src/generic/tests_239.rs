#[cfg(test)]
mod tests {
    #[cfg(feature = "count_instructions_test")]
    extern crate std;

    #[cfg(feature = "count_instructions_test")]
    #[test]
    fn count_optimizer_hide_instructions() -> std::io::Result<()> {
        use super::{Word, optimizer_hide};
        use count_instructions::count_instructions;

        fn count() -> std::io::Result<usize> {
            // If optimizer_hide does not work, constant propagation and folding
            // will make this identical to count_optimized() below.
            let mut count = 0;
            assert_eq!(
                10 as Word,
                count_instructions(
                    || optimizer_hide(1)
                        + optimizer_hide(2)
                        + optimizer_hide(3)
                        + optimizer_hide(4),
                    |_| count += 1
                )?
            );
            Ok(count)
        }

        fn count_optimized() -> std::io::Result<usize> {
            #[inline(always)]
            fn inline_identity(value: Word) -> Word {
                value
            }

            let mut count = 0;
            assert_eq!(
                10 as Word,
                count_instructions(
                    || inline_identity(1)
                        + inline_identity(2)
                        + inline_identity(3)
                        + inline_identity(4),
                    |_| count += 1
                )?
            );
            Ok(count)
        }

        assert!(count()? > count_optimized()?);
        Ok(())
    }
}
