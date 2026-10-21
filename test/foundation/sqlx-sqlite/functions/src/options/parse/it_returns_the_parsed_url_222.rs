#[test]
fn it_returns_the_parsed_url() -> Result<(), Error> {
    let url = "sqlite://test.db?mode=rw&cache=shared";
    let options: SqliteConnectOptions = url.parse()?;

    let expected_url = Url::parse(url).unwrap();
    assert_eq!(options.build_url(), expected_url);

    Ok(())
}
