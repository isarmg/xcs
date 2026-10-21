#[cfg(test)]
mod tests {
    use super::{Id, SpanStack};

    #[test]
    fn pop_last_span() {
        let mut stack = SpanStack::default();
        let id = Id::from_u64(1);
        stack.push(id.clone());

        assert!(stack.pop(&id));
    }

    #[test]
    fn pop_first_span() {
        let mut stack = SpanStack::default();
        stack.push(Id::from_u64(1));
        stack.push(Id::from_u64(2));

        let id = Id::from_u64(1);
        assert!(stack.pop(&id));
    }
}
