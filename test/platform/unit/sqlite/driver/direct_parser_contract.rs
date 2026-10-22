#[test]
fn direct_parser_preserves_paths_percent_form_options_and_errors() -> Result<(), Error> {
    for value in [
        "plain.sqlite",
        "sqlite:plain.sqlite",
        "sqlite://plain.sqlite",
    ] {
        let options: SqliteConnectOptions = value.parse()?;
        assert_eq!(&*options.filename, Path::new("plain.sqlite"));
    }
    let options: SqliteConnectOptions = "sqlite:///tmp/日本%20%3F%23%25.db?mode=ro&cache=private&immutable=true&vfs=custom+vfs%2Bname".parse()?;
    assert_eq!(&*options.filename, Path::new("/tmp/日本 ?#%.db"));
    assert!(options.read_only);
    assert!(!options.shared_cache);
    assert!(options.immutable);
    assert_eq!(options.vfs.as_deref(), Some("custom vfs+name"));
    // Preserve upstream percent handling and left-to-right duplicate option semantics.
    let options: SqliteConnectOptions =
        "literal%zz.db?mode=rwc&mode=rw&cache=shared&cache=private&immutable=1&immutable=0"
            .parse()?;
    assert_eq!(&*options.filename, Path::new("literal%zz.db"));
    assert!(options.create_if_missing);
    assert!(!options.shared_cache);
    assert!(!options.immutable);
    for value in [
        "sqlite:%FF.db",
        "sqlite:file.db?unknown=1",
        "sqlite:file.db?mode=bad",
        "sqlite:file.db?cache=bad",
        "sqlite:file.db?immutable=bad",
    ] {
        assert!(
            matches!(
                value.parse::<SqliteConnectOptions>(),
                Err(Error::Configuration(_))
            ),
            "{value}"
        );
    }
    Ok(())
}
