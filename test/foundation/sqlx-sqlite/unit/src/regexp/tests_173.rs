#[cfg(test)]
mod tests {
    use sqlx::{ConnectOptions, Row};
    use std::str::FromStr;

    async fn test_db() -> crate::SqliteConnection {
        let mut conn = crate::SqliteConnectOptions::from_str("sqlite://:memory:")
            .unwrap()
            .with_regexp()
            .connect()
            .await
            .unwrap();
        sqlx::query("CREATE TABLE test (col TEXT NOT NULL)")
            .execute(&mut conn)
            .await
            .unwrap();
        for i in 0..10 {
            sqlx::query("INSERT INTO test VALUES (?)")
                .bind(format!("value {i}"))
                .execute(&mut conn)
                .await
                .unwrap();
        }
        conn
    }

    #[sqlx::test]
    async fn test_regexp_does_not_fail() {
        let mut conn = test_db().await;
        let result = sqlx::query("SELECT col FROM test WHERE col REGEXP 'foo.*bar'")
            .fetch_all(&mut conn)
            .await
            .expect("Could not execute query");
        assert!(result.is_empty());
    }

    #[sqlx::test]
    async fn test_regexp_filters_correctly() {
        let mut conn = test_db().await;

        let result = sqlx::query("SELECT col FROM test WHERE col REGEXP '.*2'")
            .fetch_all(&mut conn)
            .await
            .expect("Could not execute query");
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].get::<String, usize>(0), String::from("value 2"));

        let result = sqlx::query("SELECT col FROM test WHERE col REGEXP '^3'")
            .fetch_all(&mut conn)
            .await
            .expect("Could not execute query");
        assert!(result.is_empty());
    }

    #[sqlx::test]
    async fn test_regexp_coerces_non_text_values() {
        let mut conn = crate::SqliteConnectOptions::from_str("sqlite://:memory:")
            .unwrap()
            .with_regexp()
            .connect()
            .await
            .unwrap();

        // INTEGER coercion
        let result: Option<i32> = sqlx::query_scalar("SELECT 123 REGEXP '23'")
            .fetch_one(&mut conn)
            .await
            .unwrap();
        assert_eq!(result, Some(1));

        // REAL coercion
        let result: Option<i32> = sqlx::query_scalar("SELECT 12.5 REGEXP '12\\.5'")
            .fetch_one(&mut conn)
            .await
            .unwrap();
        assert_eq!(result, Some(1));

        // INTEGER column
        sqlx::query("CREATE TABLE int_test (x INTEGER NOT NULL)")
            .execute(&mut conn)
            .await
            .unwrap();
        sqlx::query("INSERT INTO int_test VALUES (123), (45)")
            .execute(&mut conn)
            .await
            .unwrap();
        let rows: Vec<i64> = sqlx::query_scalar("SELECT x FROM int_test WHERE x REGEXP '23'")
            .fetch_all(&mut conn)
            .await
            .unwrap();
        assert_eq!(rows, vec![123]);

        // NULL should return NULL, not match
        let result: Option<i32> = sqlx::query_scalar("SELECT NULL REGEXP '.*'")
            .fetch_one(&mut conn)
            .await
            .unwrap();
        assert_eq!(result, None);
    }

    #[sqlx::test]
    async fn test_invalid_regexp_should_fail() {
        let mut conn = test_db().await;
        let result = sqlx::query("SELECT col from test WHERE col REGEXP '(?:?)'")
            .execute(&mut conn)
            .await;
        assert!(matches!(result, Err(sqlx::Error::Database(_))));
    }
}
