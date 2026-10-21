#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn assert_unpin() {
        use std::marker::PhantomPinned;
        crate::is_unpin::<CopyBuf<'_, PhantomPinned, PhantomPinned>>();
    }
}
