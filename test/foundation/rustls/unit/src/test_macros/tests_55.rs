#[cfg(test)]
#[macro_rules_attribute::apply(test_for_each_provider)]
mod tests {
    #[test]
    fn test_each_provider() {
        std::println!("provider is {:?}", super::provider::default_provider());
    }
}
