#[test]
fn it_returns_the_parsed_url() -> Result<(), Error> {
    let url = "sqlite://test.db?mode=rw&cache=shared";
    let options: SqliteConnectOptions = url.parse()?;

    let expected_url = Url::parse(url).unwrap();
    assert_eq!(options.build_url(), expected_url);

    Ok(())
}

#[test]
fn typed_url_conversion_preserves_retained_sqlite_options() -> Result<(), Error> {
    use crate::ConnectOptions;
    let url = Url::parse(
        "sqlite:///tmp/example%20%3F%23.db?mode=ro&cache=private&immutable=true&vfs=test%2Bvfs",
    )
    .unwrap();
    let options = SqliteConnectOptions::from_url(&url)?;
    let roundtrip = SqliteConnectOptions::from_url(&options.to_url_lossy())?;
    assert_eq!(roundtrip.filename, options.filename);
    assert_eq!(roundtrip.read_only, options.read_only);
    assert_eq!(roundtrip.shared_cache, options.shared_cache);
    assert_eq!(roundtrip.immutable, options.immutable);
    assert_eq!(roundtrip.vfs, options.vfs);
    Ok(())
}
