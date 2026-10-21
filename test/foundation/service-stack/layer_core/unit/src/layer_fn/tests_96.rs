#[cfg(test)]
mod tests {
    use super::*;

    #[allow(dead_code)]
    #[test]
    fn layer_fn_has_useful_debug_impl() {
        struct WrappedService<S> {
            inner: S,
        }
        let layer = layer_fn(|svc| WrappedService { inner: svc });
        let _svc = layer.layer("foo");

        assert_eq!(
            "LayerFn { f: service_stack::layer_core::layer_fn::tests::layer_fn_has_useful_debug_impl::{{closure}} }".to_string(),
            format!("{:?}", layer),
        );
    }
}
