#[cfg(test)]
mod tests {
    use crate::{ConnectOptions, Row};
    use std::str::FromStr;

    async fn test_db() -> crate::sqlite::SqliteConnection {
        let mut conn = crate::sqlite::SqliteConnectOptions::from_str("sqlite://:memory:")
            .unwrap()
            .with_regexp()
            .connect()
            .await
            .unwrap();
        crate::query("CREATE TABLE test (col TEXT NOT NULL)")
            .execute(&mut conn)
            .await
            .unwrap();
        for i in 0..10 {
            crate::query("INSERT INTO test VALUES (?)")
                .bind(format!("value {i}"))
                .execute(&mut conn)
                .await
                .unwrap();
        }
        conn
    }

    #[test]
    fn test_regexp_does_not_fail() {
        tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap().block_on(async {
        let mut conn = test_db().await;
        let result = crate::query("SELECT col FROM test WHERE col REGEXP 'foo.*bar'")
            .fetch_all(&mut conn)
            .await
            .expect("Could not execute query");
        assert!(result.is_empty());
        });
    }

    #[test]
    fn test_regexp_filters_correctly() {
        tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap().block_on(async {
        let mut conn = test_db().await;

        let result = crate::query("SELECT col FROM test WHERE col REGEXP '.*2'")
            .fetch_all(&mut conn)
            .await
            .expect("Could not execute query");
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].get::<String, usize>(0), String::from("value 2"));

        let result = crate::query("SELECT col FROM test WHERE col REGEXP '^3'")
            .fetch_all(&mut conn)
            .await
            .expect("Could not execute query");
        assert!(result.is_empty());
        });
    }

    #[test]
    fn test_regexp_coerces_non_text_values() {
        tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap().block_on(async {
        let mut conn = crate::sqlite::SqliteConnectOptions::from_str("sqlite://:memory:")
            .unwrap()
            .with_regexp()
            .connect()
            .await
            .unwrap();

        // INTEGER coercion
        let result: Option<i32> = crate::query_scalar("SELECT 123 REGEXP '23'")
            .fetch_one(&mut conn)
            .await
            .unwrap();
        assert_eq!(result, Some(1));

        // REAL coercion
        let result: Option<i32> = crate::query_scalar("SELECT 12.5 REGEXP '12\\.5'")
            .fetch_one(&mut conn)
            .await
            .unwrap();
        assert_eq!(result, Some(1));

        // INTEGER column
        crate::query("CREATE TABLE int_test (x INTEGER NOT NULL)")
            .execute(&mut conn)
            .await
            .unwrap();
        crate::query("INSERT INTO int_test VALUES (123), (45)")
            .execute(&mut conn)
            .await
            .unwrap();
        let rows: Vec<i64> = crate::query_scalar("SELECT x FROM int_test WHERE x REGEXP '23'")
            .fetch_all(&mut conn)
            .await
            .unwrap();
        assert_eq!(rows, vec![123]);

        // NULL should return NULL, not match
        let result: Option<i32> = crate::query_scalar("SELECT NULL REGEXP '.*'")
            .fetch_one(&mut conn)
            .await
            .unwrap();
        assert_eq!(result, None);
        });
    }

    #[test]
    fn test_invalid_regexp_should_fail() {
        tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap().block_on(async {
        let mut conn = test_db().await;
        let result = crate::query("SELECT col from test WHERE col REGEXP '(?:?)'")
            .execute(&mut conn)
            .await;
        assert!(matches!(result, Err(crate::Error::Database(_))));
        });
    }
}
