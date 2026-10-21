#[test]
fn test_root_block_columns_has_types() {
    use crate::SqliteConnectOptions;
    use std::str::FromStr;
    let conn_options = SqliteConnectOptions::from_str("sqlite::memory:").unwrap();
    let mut conn = super::EstablishParams::from_options(&conn_options)
        .unwrap()
        .establish()
        .unwrap();

    assert!(execute::iter(
        &mut conn,
        r"CREATE TABLE t(a INTEGER PRIMARY KEY, b_null TEXT NULL, b TEXT NOT NULL);",
        None,
        false
    )
    .unwrap()
    .next()
    .is_some());
    assert!(
        execute::iter(&mut conn, r"CREATE INDEX i1 on t (a,b_null);", None, false)
            .unwrap()
            .next()
            .is_some()
    );
    assert!(execute::iter(
        &mut conn,
        r"CREATE UNIQUE INDEX i2 on t (a,b_null);",
        None,
        false
    )
    .unwrap()
    .next()
    .is_some());
    assert!(execute::iter(
        &mut conn,
        r"CREATE TABLE t2(a INTEGER NOT NULL, b_null NUMERIC NULL, b NUMERIC NOT NULL);",
        None,
        false
    )
    .unwrap()
    .next()
    .is_some());
    assert!(execute::iter(
        &mut conn,
        r"CREATE INDEX t2i1 on t2 (a,b_null);",
        None,
        false
    )
    .unwrap()
    .next()
    .is_some());
    assert!(execute::iter(
        &mut conn,
        r"CREATE UNIQUE INDEX t2i2 on t2 (a,b);",
        None,
        false
    )
    .unwrap()
    .next()
    .is_some());

    assert!(execute::iter(
        &mut conn,
        r"CREATE TEMPORARY TABLE t3(a TEXT PRIMARY KEY, b REAL NOT NULL, b_null REAL NULL);",
        None,
        false
    )
    .unwrap()
    .next()
    .is_some());

    let table_block_nums: HashMap<String, (i64,i64)> = execute::iter(
        &mut conn,
        r"select name, 0 db_seq, rootpage from main.sqlite_schema UNION ALL select name, 1 db_seq, rootpage from temp.sqlite_schema",
        None,
        false,
    )
    .unwrap()
    .filter_map(|res| res.map(|either| either.right()).transpose())
    .map(|row| FromRow::from_row(row.as_ref().unwrap()))
    .map(|row| row.map(|(name,seq,block)|(name,(seq,block))))
    .collect::<Result<HashMap<_, _>, Error>>()
    .unwrap();

    let root_block_cols = root_block_columns(&mut conn).unwrap();

    // there should be 7 tables/indexes created explicitly, plus 1 autoindex for t3
    assert_eq!(8, root_block_cols.len());

    //prove that we have some information for each table & index
    for (name, db_seq_block) in dbg!(&table_block_nums) {
        assert!(
            root_block_cols.contains_key(db_seq_block),
            "{:?}",
            (name, db_seq_block)
        );
    }

    //prove that each block has the correct information
    {
        let table_db_block = table_block_nums["t"];
        assert_eq!(
            Some(&ColumnType::Single {
                datatype: DataType::Integer,
                nullable: Some(true) //sqlite primary key columns are nullable unless declared not null
            }),
            root_block_cols[&table_db_block].get(&0)
        );
        assert_eq!(
            Some(&ColumnType::Single {
                datatype: DataType::Text,
                nullable: Some(true)
            }),
            root_block_cols[&table_db_block].get(&1)
        );
        assert_eq!(
            Some(&ColumnType::Single {
                datatype: DataType::Text,
                nullable: Some(false)
            }),
            root_block_cols[&table_db_block].get(&2)
        );
    }

    {
        let table_db_block = table_block_nums["i1"];
        assert_eq!(
            Some(&ColumnType::Single {
                datatype: DataType::Integer,
                nullable: Some(true) //sqlite primary key columns are nullable unless declared not null
            }),
            root_block_cols[&table_db_block].get(&0)
        );
        assert_eq!(
            Some(&ColumnType::Single {
                datatype: DataType::Text,
                nullable: Some(true)
            }),
            root_block_cols[&table_db_block].get(&1)
        );
    }

    {
        let table_db_block = table_block_nums["i2"];
        assert_eq!(
            Some(&ColumnType::Single {
                datatype: DataType::Integer,
                nullable: Some(true) //sqlite primary key columns are nullable unless declared not null
            }),
            root_block_cols[&table_db_block].get(&0)
        );
        assert_eq!(
            Some(&ColumnType::Single {
                datatype: DataType::Text,
                nullable: Some(true)
            }),
            root_block_cols[&table_db_block].get(&1)
        );
    }

    {
        let table_db_block = table_block_nums["t2"];
        assert_eq!(
            Some(&ColumnType::Single {
                datatype: DataType::Integer,
                nullable: Some(false)
            }),
            root_block_cols[&table_db_block].get(&0)
        );
        assert_eq!(
            Some(&ColumnType::Single {
                datatype: DataType::Null,
                nullable: Some(true)
            }),
            root_block_cols[&table_db_block].get(&1)
        );
        assert_eq!(
            Some(&ColumnType::Single {
                datatype: DataType::Null,
                nullable: Some(false)
            }),
            root_block_cols[&table_db_block].get(&2)
        );
    }

    {
        let table_db_block = table_block_nums["t2i1"];
        assert_eq!(
            Some(&ColumnType::Single {
                datatype: DataType::Integer,
                nullable: Some(false)
            }),
            root_block_cols[&table_db_block].get(&0)
        );
        assert_eq!(
            Some(&ColumnType::Single {
                datatype: DataType::Null,
                nullable: Some(true)
            }),
            root_block_cols[&table_db_block].get(&1)
        );
    }

    {
        let table_db_block = table_block_nums["t2i2"];
        assert_eq!(
            Some(&ColumnType::Single {
                datatype: DataType::Integer,
                nullable: Some(false)
            }),
            root_block_cols[&table_db_block].get(&0)
        );
        assert_eq!(
            Some(&ColumnType::Single {
                datatype: DataType::Null,
                nullable: Some(false)
            }),
            root_block_cols[&table_db_block].get(&1)
        );
    }

    {
        let table_db_block = table_block_nums["t3"];
        assert_eq!(
            Some(&ColumnType::Single {
                datatype: DataType::Text,
                nullable: Some(true)
            }),
            root_block_cols[&table_db_block].get(&0)
        );
        assert_eq!(
            Some(&ColumnType::Single {
                datatype: DataType::Float,
                nullable: Some(false)
            }),
            root_block_cols[&table_db_block].get(&1)
        );
        assert_eq!(
            Some(&ColumnType::Single {
                datatype: DataType::Float,
                nullable: Some(true)
            }),
            root_block_cols[&table_db_block].get(&2)
        );
    }
}
