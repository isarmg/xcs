#[cfg(test)]
mod public_api {
    use super::{Evaluation, HierarchicalOutput, ListOutput};

    // These are returned from the public API, so dropping a derive breaks downstream `{:?}`.
    #[test]
    fn output_types_are_debug() {
        fn assert_debug<T: std::fmt::Debug>() {}
        assert_debug::<Evaluation>();
        assert_debug::<ListOutput<'_>>();
        assert_debug::<HierarchicalOutput<'_>>();
    }
}
