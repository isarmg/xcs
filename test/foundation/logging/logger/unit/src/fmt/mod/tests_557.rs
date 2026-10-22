#[cfg(test)]
mod tests {
    use super::*;

    use log::{Level, Record};

    fn write_record(record: Record<'_>, fmt: ConfigurableFormatWriter<'_>) -> String {
        let buf = fmt.buf.buf.clone();

        fmt.write(&record).expect("failed to write record");

        let buf = buf.borrow();
        String::from_utf8(buf.as_bytes().to_vec()).expect("failed to read record")
    }

    fn write_target(target: &str, fmt: ConfigurableFormatWriter<'_>) -> String {
        write_record(
            Record::builder()
                .args(format_args!("log\nmessage"))
                .level(Level::Info)
                .file(Some("test.rs"))
                .line(Some(144))
                .module_path(Some("test::path"))
                .target(target)
                .build(),
            fmt,
        )
    }

    fn write(fmt: ConfigurableFormatWriter<'_>) -> String {
        write_target("", fmt)
    }

    fn formatter() -> Formatter {
        let writer = crate::writer::Builder::new()
            .write_style(WriteStyle::Never)
            .build();

        Formatter::new(&writer)
    }

    #[test]
    fn format_with_header() {
        let mut f = formatter();

        let written = write(ConfigurableFormatWriter {
            format: &ConfigurableFormat {
                module_path: true,
                target: false,
                level: true,
                source_file: false,
                source_line_number: false,
                #[cfg(feature = "kv")]
                kv_format: Some(Box::new(hidden_kv_format)),
                indent: None,
                suffix: "\n",
            },
            written_header_value: false,
            buf: &mut f,
        });

        assert_eq!("[INFO  test::path] log\nmessage\n", written);
    }

    #[test]
    fn format_no_header() {
        let mut f = formatter();

        let written = write(ConfigurableFormatWriter {
            format: &ConfigurableFormat {
                module_path: false,
                target: false,
                level: false,
                source_file: false,
                source_line_number: false,
                #[cfg(feature = "kv")]
                kv_format: Some(Box::new(hidden_kv_format)),
                indent: None,
                suffix: "\n",
            },
            written_header_value: false,
            buf: &mut f,
        });

        assert_eq!("log\nmessage\n", written);
    }

    #[test]
    fn format_indent_spaces() {
        let mut f = formatter();

        let written = write(ConfigurableFormatWriter {
            format: &ConfigurableFormat {
                module_path: true,
                target: false,
                level: true,
                source_file: false,
                source_line_number: false,
                #[cfg(feature = "kv")]
                kv_format: Some(Box::new(hidden_kv_format)),
                indent: Some(4),
                suffix: "\n",
            },
            written_header_value: false,
            buf: &mut f,
        });

        assert_eq!("[INFO  test::path] log\n    message\n", written);
    }

    #[test]
    fn format_indent_zero_spaces() {
        let mut f = formatter();

        let written = write(ConfigurableFormatWriter {
            format: &ConfigurableFormat {
                module_path: true,
                target: false,
                level: true,
                source_file: false,
                source_line_number: false,
                #[cfg(feature = "kv")]
                kv_format: Some(Box::new(hidden_kv_format)),
                indent: Some(0),
                suffix: "\n",
            },
            written_header_value: false,
            buf: &mut f,
        });

        assert_eq!("[INFO  test::path] log\nmessage\n", written);
    }

    #[test]
    fn format_indent_spaces_no_header() {
        let mut f = formatter();

        let written = write(ConfigurableFormatWriter {
            format: &ConfigurableFormat {
                module_path: false,
                target: false,
                level: false,
                source_file: false,
                source_line_number: false,
                #[cfg(feature = "kv")]
                kv_format: Some(Box::new(hidden_kv_format)),
                indent: Some(4),
                suffix: "\n",
            },
            written_header_value: false,
            buf: &mut f,
        });

        assert_eq!("log\n    message\n", written);
    }

    #[test]
    fn format_suffix() {
        let mut f = formatter();

        let written = write(ConfigurableFormatWriter {
            format: &ConfigurableFormat {
                module_path: false,
                target: false,
                level: false,
                source_file: false,
                source_line_number: false,
                #[cfg(feature = "kv")]
                kv_format: Some(Box::new(hidden_kv_format)),
                indent: None,
                suffix: "\n\n",
            },
            written_header_value: false,
            buf: &mut f,
        });

        assert_eq!("log\nmessage\n\n", written);
    }

    #[test]
    fn format_suffix_with_indent() {
        let mut f = formatter();

        let written = write(ConfigurableFormatWriter {
            format: &ConfigurableFormat {
                module_path: false,
                target: false,
                level: false,
                source_file: false,
                source_line_number: false,
                #[cfg(feature = "kv")]
                kv_format: Some(Box::new(hidden_kv_format)),
                indent: Some(4),
                suffix: "\n\n",
            },
            written_header_value: false,
            buf: &mut f,
        });

        assert_eq!("log\n\n    message\n\n", written);
    }

    #[test]
    fn format_target() {
        let mut f = formatter();

        let written = write_target(
            "target",
            ConfigurableFormatWriter {
                format: &ConfigurableFormat {
                    module_path: true,
                    target: true,
                    level: true,
                    source_file: false,
                    source_line_number: false,
                    #[cfg(feature = "kv")]
                    kv_format: Some(Box::new(hidden_kv_format)),
                    indent: None,
                    suffix: "\n",
                },
                written_header_value: false,
                buf: &mut f,
            },
        );

        assert_eq!("[INFO  test::path target] log\nmessage\n", written);
    }

    #[test]
    fn format_empty_target() {
        let mut f = formatter();

        let written = write(ConfigurableFormatWriter {
            format: &ConfigurableFormat {
                module_path: true,
                target: true,
                level: true,
                source_file: false,
                source_line_number: false,
                #[cfg(feature = "kv")]
                kv_format: Some(Box::new(hidden_kv_format)),
                indent: None,
                suffix: "\n",
            },
            written_header_value: false,
            buf: &mut f,
        });

        assert_eq!("[INFO  test::path] log\nmessage\n", written);
    }

    #[test]
    fn format_no_target() {
        let mut f = formatter();

        let written = write_target(
            "target",
            ConfigurableFormatWriter {
                format: &ConfigurableFormat {
                    module_path: true,
                    target: false,
                    level: true,
                    source_file: false,
                    source_line_number: false,
                    #[cfg(feature = "kv")]
                    kv_format: Some(Box::new(hidden_kv_format)),
                    indent: None,
                    suffix: "\n",
                },
                written_header_value: false,
                buf: &mut f,
            },
        );

        assert_eq!("[INFO  test::path] log\nmessage\n", written);
    }

    #[test]
    fn format_with_source_file_and_line_number() {
        let mut f = formatter();

        let written = write(ConfigurableFormatWriter {
            format: &ConfigurableFormat {
                module_path: false,
                target: false,
                level: true,
                source_file: true,
                source_line_number: true,
                #[cfg(feature = "kv")]
                kv_format: Some(Box::new(hidden_kv_format)),
                indent: None,
                suffix: "\n",
            },
            written_header_value: false,
            buf: &mut f,
        });

        assert_eq!("[INFO  test.rs:144] log\nmessage\n", written);
    }

    #[cfg(feature = "kv")]
    #[test]
    fn format_kv_default() {
        let kvs = &[("a", 1u32), ("b", 2u32)][..];
        let mut f = formatter();
        let record = Record::builder()
            .args(format_args!("log message"))
            .level(Level::Info)
            .module_path(Some("test::path"))
            .key_values(&kvs)
            .build();

        let written = write_record(
            record,
            ConfigurableFormatWriter {
                format: &ConfigurableFormat {
                    module_path: false,
                    target: false,
                    level: true,
                    source_file: false,
                    source_line_number: false,
                    kv_format: Some(Box::new(default_kv_format)),
                    indent: None,
                    suffix: "\n",
                },
                written_header_value: false,
                buf: &mut f,
            },
        );

        assert_eq!("[INFO ] log message a=1 b=2\n", written);
    }

    #[cfg(feature = "kv")]
    #[test]
    fn format_kv_default_full() {
        let kvs = &[("a", 1u32), ("b", 2u32)][..];
        let mut f = formatter();
        let record = Record::builder()
            .args(format_args!("log\nmessage"))
            .level(Level::Info)
            .module_path(Some("test::path"))
            .target("target")
            .file(Some("test.rs"))
            .line(Some(42))
            .key_values(&kvs)
            .build();

        let written = write_record(
            record,
            ConfigurableFormatWriter {
                format: &ConfigurableFormat {
                    module_path: true,
                    target: true,
                    level: true,
                    source_file: true,
                    source_line_number: true,
                    kv_format: Some(Box::new(default_kv_format)),
                    indent: None,
                    suffix: "\n",
                },
                written_header_value: false,
                buf: &mut f,
            },
        );

        assert_eq!(
            "[INFO  test::path test.rs:42 target] log\nmessage a=1 b=2\n",
            written
        );
    }
}
