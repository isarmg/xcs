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

#[test]
fn prepared_queries_reject_nul_before_execution() -> Result<(), Error> {
    use crate::sqlite::statement::VirtualStatement;
    use crate::{Connection, Executor, SqliteConnection};

    let invalid_sql = [
        "\0",
        "SELECT 1\0",
        "SELECT 1;\0 SELECT 2",
        "-- comment\0",
        "SELECT '\0'",
        "INSERT INTO retained VALUES (1);\0 INSERT INTO retained VALUES (2)",
    ];
    // Check before entering the worker: a regression must fail rather than
    // hang the test process on SQLite's zero-progress NUL terminator.
    for sql in invalid_sql {
        for persistent in [false, true] {
            assert!(
                matches!(
                    VirtualStatement::new(sql, persistent),
                    Err(Error::Protocol(_))
                ),
                "accepted NUL SQL: {sql:?}"
            );
        }
    }

    futures::executor::block_on(async {
        let mut connection = SqliteConnection::connect("sqlite::memory:").await?;
        crate::query("CREATE TABLE retained (value INTEGER)")
            .execute(&mut connection)
            .await?;
        for _ in 0..2 {
            for sql in invalid_sql {
                assert!(matches!(
                    connection.prepare(crate::SqlStr::from_static(sql)).await,
                    Err(Error::Protocol(_))
                ));
                for persistent in [false, true] {
                    assert!(matches!(
                        crate::query(sql)
                            .persistent(persistent)
                            .execute(&mut connection)
                            .await,
                        Err(Error::Protocol(_))
                    ));
                }
                assert!(matches!(
                    crate::raw_sql(sql).execute(&mut connection).await,
                    Err(Error::Protocol(_))
                ));
            }
        }
        // Reject the whole input before executing a valid prefix. The worker
        // and its statement cache must remain usable after the errors.
        assert_eq!(
            crate::query_scalar::<_, i64>("SELECT count(*) FROM retained")
                .fetch_one(&mut connection)
                .await?,
            0
        );
        crate::raw_sql(" ; -- empty statement\n INSERT INTO retained VALUES (3); ")
            .execute(&mut connection)
            .await?;
        assert_eq!(
            crate::query_scalar::<_, i64>("SELECT value FROM retained")
                .fetch_one(&mut connection)
                .await?,
            3
        );
        // NUL is valid in bound data; only the SQL source is rejected.
        assert_eq!(
            crate::query_scalar::<_, String>("SELECT $1")
                .bind("left\0right")
                .fetch_one(&mut connection)
                .await?,
            "left\0right"
        );
        connection.close().await?;
        Ok(())
    })
}

#[test]
fn nul_filenames_are_rejected_before_uri_encoding() -> Result<(), Error> {
    use crate::sqlite::connection::establish::EstablishParams;
    use crate::{Connection, SqliteConnection};

    futures::executor::block_on(async {
        for immutable in [false, true] {
            for vfs in [None, Some("unix")] {
                let mut options = SqliteConnectOptions::new()
                    .filename(":memory:\0suffix")
                    .immutable(immutable)
                    .create_if_missing(true);
                if let Some(vfs) = vfs {
                    options = options.vfs(vfs);
                }
                assert!(matches!(
                    EstablishParams::from_options(&options),
                    Err(Error::Io(error)) if error.kind() == std::io::ErrorKind::InvalidData
                ));
                assert!(matches!(
                    SqliteConnection::connect_with(&options).await,
                    Err(Error::Io(error)) if error.kind() == std::io::ErrorKind::InvalidData
                ));
            }
        }
        for options in [
            SqliteConnectOptions::new().filename("file::memory:%00suffix"),
            SqliteConnectOptions::new().filename("file::memory:?mode=memory%00invalid"),
            SqliteConnectOptions::new().filename("file::memory:?vfs=unix%00invalid"),
            SqliteConnectOptions::new().vfs("unix\0invalid"),
        ] {
            assert!(matches!(
                EstablishParams::from_options(&options),
                Err(Error::Io(error)) if error.kind() == std::io::ErrorKind::InvalidData
            ));
            assert!(matches!(
                SqliteConnection::connect_with(&options).await,
                Err(Error::Io(error)) if error.kind() == std::io::ErrorKind::InvalidData
            ));
        }
        // Only a decoded NUL is invalid: percent signs in ordinary paths,
        // escaped percent signs in URIs, and ignored fragments stay valid.
        for options in [
            SqliteConnectOptions::new(),
            SqliteConnectOptions::new().filename("file::memory:"),
            SqliteConnectOptions::new().filename("file::memory:#ignored%00fragment"),
            SqliteConnectOptions::new().filename("file:literal%2500?mode=memory"),
            SqliteConnectOptions::new()
                .filename("literal%00.sqlite")
                .in_memory(true),
            SqliteConnectOptions::new()
                .filename("literal%00.sqlite")
                .in_memory(true)
                .immutable(true),
        ] {
            SqliteConnection::connect_with(&options)
                .await?
                .close()
                .await?;
        }
        Ok(())
    })
}
