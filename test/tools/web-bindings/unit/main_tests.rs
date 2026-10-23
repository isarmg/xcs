use super::*;

#[test]
fn only_the_browser_binding_contract_is_accepted() {
    let arguments = [
        "wasm-bindgen",
        "--target",
        "web",
        "--no-typescript",
        "--out-name",
        "zxs",
        "--out-dir",
        "out",
        "app.wasm",
    ];
    assert!(Args::try_parse_from(arguments).is_ok());
    for target in [
        "bundler",
        "nodejs",
        "nodejs-module",
        "deno",
        "no-modules",
        "emscripten",
        "module",
    ] {
        let mut changed = arguments;
        changed[2] = target;
        assert!(Args::try_parse_from(changed).is_err(), "{target}");
    }
    assert!(Args::try_parse_from(
        arguments
            .into_iter()
            .filter(|arg| *arg != "--no-typescript")
    )
    .is_err());
}

#[test]
fn emscripten_markers_fail_before_descriptor_processing() {
    for name in [
        "__wasm_bindgen_emscripten_marker",
        "__wasm_bindgen_emscripten_jspi_marker",
    ] {
        let mut wasm = b"\0asm\x01\0\0\0".to_vec();
        // Each name and section fit in a single unsigned LEB128 byte.
        wasm.extend([0, (name.len() + 1) as u8, name.len() as u8]);
        wasm.extend(name.as_bytes());
        let mut generator = Bindgen::new();
        generator.input_bytes("unsupported", wasm);
        let error = generator
            .generate_output()
            .err()
            .expect("must reject unsupported backend");
        assert!(
            error
                .to_string()
                .contains("unsupported Emscripten module marker"),
            "{error}"
        );
    }
}

#[test]
fn help_and_version_do_not_require_a_wasm_input() {
    assert!(run(["wasm-bindgen", "--version"]).is_ok());
    assert!(run(["wasm-bindgen", "--help"]).is_ok());
}

#[test]
fn output_name_cannot_escape_the_directory_or_change_generated_javascript() {
    for name in ["zxs", "web-app_2", "0"] {
        assert!(output_name(name).is_ok(), "{name:?}");
    }
    for name in [
        "",
        ".",
        "..",
        "../escaped",
        "/absolute",
        "nested/file",
        "nested\\file",
        "C:stream",
        "bad'url",
        "bad\"url",
        "bad\nurl",
        "app.js",
    ] {
        // Argument parsing fails before any Wasm input is read or generated
        // output can be written, including for a nonexistent source file.
        let error = run([
            "wasm-bindgen",
            "--target",
            "web",
            "--no-typescript",
            "--out-name",
            name,
            "--out-dir",
            "out",
            "nonexistent-input.wasm",
        ])
        .unwrap_err();
        assert!(
            error.to_string().contains("output name must contain"),
            "{name:?}: {error}"
        );
    }
}
