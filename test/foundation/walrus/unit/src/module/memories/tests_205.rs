#[cfg(test)]
mod tests {
    use crate::Module;

    #[test]
    fn memories_len() {
        let mut module = Module::default();
        assert_eq!(module.memories.len(), 0);

        module.memories.add_local(false, false, 0, Some(1024), None);
        assert_eq!(module.memories.len(), 1);

        module
            .memories
            .add_local(true, true, 1024, Some(2048), Some(16));
        assert_eq!(module.memories.len(), 2);
    }
}
