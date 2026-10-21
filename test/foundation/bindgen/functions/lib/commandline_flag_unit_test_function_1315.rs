/// Test `command_line_flag` function.
#[test]
fn commandline_flag_unit_test_function() {
    //Test 1
    let bindings = builder();
    let command_line_flags = bindings.command_line_flags();

    let test_cases = [
        "--rust-target",
        "--no-derive-default",
        "--generate",
        "functions,types,vars,methods,constructors,destructors",
    ]
    .iter()
    .map(|&x| x.into())
    .collect::<Vec<String>>();

    assert!(test_cases.iter().all(|x| command_line_flags.contains(x)));

    //Test 2
    let bindings = builder()
        .header("input_header")
        .allowlist_type("Distinct_Type")
        .allowlist_function("safe_function");

    let command_line_flags = bindings.command_line_flags();
    let test_cases = [
        "--rust-target",
        "input_header",
        "--no-derive-default",
        "--generate",
        "functions,types,vars,methods,constructors,destructors",
        "--allowlist-type",
        "Distinct_Type",
        "--allowlist-function",
        "safe_function",
    ]
    .iter()
    .map(|&x| x.into())
    .collect::<Vec<String>>();
    println!("{command_line_flags:?}");

    assert!(test_cases.iter().all(|x| command_line_flags.contains(x)));
}
