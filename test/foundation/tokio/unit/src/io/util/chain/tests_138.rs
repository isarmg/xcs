#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn assert_unpin() {
        crate::is_unpin::<Chain<(), ()>>();
    }
}
