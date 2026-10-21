#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_eh_version_none() {
        let wat = r#"
            (module
                (func $foo
                    i32.const 1
                    drop
                )
                (export "foo" (func $foo))
            )
        "#;
        let module = parse_wat(wat);
        assert_eq!(
            detect_exception_handling_version(&module, false),
            ExceptionHandlingVersion::None
        );
    }

    #[test]
    fn detect_eh_version_legacy() {
        // Legacy EH uses try/catch_all/end syntax
        let wat = r#"
            (module
                (func $foo
                    try
                        i32.const 1
                        drop
                    catch_all
                    end
                )
                (export "foo" (func $foo))
            )
        "#;
        let module = parse_wat(wat);
        assert_eq!(
            detect_exception_handling_version(&module, false),
            ExceptionHandlingVersion::Legacy
        );
    }

    #[test]
    fn detect_eh_version_modern() {
        // Modern EH uses try_table with catch_all targeting a block label
        let wat = r#"
            (module
                (func $foo
                    (block $catch
                        (try_table (catch_all $catch)
                            i32.const 1
                            drop
                        )
                    )
                )
                (export "foo" (func $foo))
            )
        "#;
        let module = parse_wat(wat);
        assert_eq!(
            detect_exception_handling_version(&module, false),
            ExceptionHandlingVersion::Modern
        );
    }

    #[test]
    fn detect_eh_version_both_prefers_modern() {
        // If a module somehow has both, we prefer modern
        let wat = r#"
            (module
                (func $foo
                    try
                        i32.const 1
                        drop
                    catch_all
                    end
                )
                (func $bar
                    (block $catch
                        (try_table (catch_all $catch)
                            i32.const 1
                            drop
                        )
                    )
                )
                (export "foo" (func $foo))
                (export "bar" (func $bar))
            )
        "#;
        let module = parse_wat(wat);
        assert_eq!(
            detect_exception_handling_version(&module, false),
            ExceptionHandlingVersion::Modern
        );
    }
}
