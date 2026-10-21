#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_layouts() {
        use crate::backend::c;

        check_renamed_struct!(Wait, futex_waitv, val, uaddr, flags, __reserved);
    }
}
