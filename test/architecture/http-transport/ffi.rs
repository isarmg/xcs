use http_transport::ffi::*;

#[test]
fn opaque_options_are_distinct_live_allocations() {
    assert!(std::mem::size_of::<hyper_clientconn_options>() > 0);
    let first = hyper_clientconn_options_new();
    let second = hyper_clientconn_options_new();
    assert!(!first.is_null() && !second.is_null());
    assert_ne!(first, second);
    hyper_clientconn_options_set_preserve_header_case(first, 1);
    hyper_clientconn_options_set_preserve_header_order(first, 1);
    hyper_clientconn_options_http1_allow_multiline_headers(first, 1);
    hyper_clientconn_options_free(first);
    hyper_clientconn_options_free(second);
}

#[test]
fn retained_http2_abi_reports_unsupported_without_consuming_options() {
    let options = hyper_clientconn_options_new();
    assert!(!options.is_null());
    for enabled in [0, 1] {
        assert!(matches!(hyper_clientconn_options_http2(options, enabled),
            hyper_code::HYPERE_FEATURE_NOT_ENABLED));
    }
    hyper_clientconn_options_set_preserve_header_case(options, 1);
    hyper_clientconn_options_free(options);
}
