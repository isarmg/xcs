#[cfg(feature = "std")]
#[test]
fn check_description() {
    let expect = if cfg!(windows) {
        "Incorrect function."
    } else if cfg!(target_os = "illumos") {
        "Not owner"
    } else if cfg!(target_os = "wasi") || cfg!(target_os = "emscripten") {
        "Argument list too long"
    } else if cfg!(target_os = "haiku") {
        "Operation not allowed"
    } else if cfg!(target_os = "vxworks") {
        "operation not permitted"
    } else {
        "Operation not permitted"
    };

    let errno_code = if cfg!(target_os = "haiku") {
        -2147483633
    } else if cfg!(target_os = "hurd") {
        1073741825
    } else {
        1
    };
    set_errno(Errno(errno_code));

    assert_eq!(errno().to_string(), expect);
    assert_eq!(
        format!("{:?}", errno()),
        format!(
            "Errno {{ code: {}, description: Some({:?}) }}",
            errno_code, expect
        )
    );
}
