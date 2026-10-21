#[test]
fn test_explain() {
    use crate::sqlite::SqliteConnectOptions;
    use std::str::FromStr;
    let conn_options = SqliteConnectOptions::from_str("sqlite::memory:").unwrap();
    let mut conn = super::EstablishParams::from_options(&conn_options)
        .unwrap()
        .establish()
        .unwrap();

    assert!(execute::iter(
        &mut conn,
        r"CREATE TABLE an_alias(a INTEGER PRIMARY KEY);",
        None,
        false
    )
    .unwrap()
    .next()
    .is_some());

    assert!(execute::iter(
        &mut conn,
        r"CREATE TABLE not_an_alias(a INT PRIMARY KEY);",
        None,
        false
    )
    .unwrap()
    .next()
    .is_some());

    assert!(
        if let Ok((ty, nullable)) = explain(&mut conn, "SELECT * FROM an_alias") {
            ty == [SqliteTypeInfo(DataType::Integer)] && nullable == [Some(false)]
        } else {
            false
        }
    );
    assert!(
        if let Ok((ty, nullable)) = explain(&mut conn, "SELECT * FROM not_an_alias") {
            ty == [SqliteTypeInfo(DataType::Integer)] && nullable == [Some(true)]
        } else {
            false
        }
    );
    assert!(
        if let Ok((ty, nullable)) = explain(&mut conn, "SELECT rowid FROM an_alias") {
            ty == [SqliteTypeInfo(DataType::Integer)] && nullable == [Some(false)]
        } else {
            false
        }
    );
    assert!(
        if let Ok((ty, nullable)) = explain(&mut conn, "SELECT rowid FROM not_an_alias") {
            ty == [SqliteTypeInfo(DataType::Integer)] && nullable == [Some(false)]
        } else {
            false
        }
    );
}
