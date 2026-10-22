#[cfg(test)]
mod tests {
    use super::*;

    // Anchor names from externally registered resources bypass `anchorString` validation, so a
    // name or resource URI may contain the join delimiters themselves.
    #[test]
    fn specialized_key_is_injective_for_components_holding_delimiters() {
        let key: Arc<str> = Arc::from("https://example.com/target");
        let name_with_at: DynamicEnv =
            Arc::from(vec![(Arc::<str>::from("a@r"), Arc::<str>::from("x"))]);
        let resource_with_at: DynamicEnv =
            Arc::from(vec![(Arc::<str>::from("a"), Arc::<str>::from("r@x"))]);
        assert_ne!(
            specialized_key(&key, &name_with_at),
            specialized_key(&key, &resource_with_at)
        );

        let key_with_delimiter: Arc<str> = Arc::from("k|dyn=a@r");
        let one_binding: DynamicEnv =
            Arc::from(vec![(Arc::<str>::from("b"), Arc::<str>::from("s"))]);
        let plain_key: Arc<str> = Arc::from("k");
        let two_bindings: DynamicEnv = Arc::from(vec![
            (Arc::<str>::from("a"), Arc::<str>::from("r")),
            (Arc::<str>::from("b"), Arc::<str>::from("s")),
        ]);
        assert_ne!(
            specialized_key(&key_with_delimiter, &one_binding),
            specialized_key(&plain_key, &two_bindings)
        );
    }
}
