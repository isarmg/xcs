//! Rebuildable SQLite projection; never an authority for publication.
use super::Engine;
use crate::*;
use sqlx::{Connection, Executor};

impl Engine {
    pub(crate) fn rebuild_index(&self) -> Result<()> {
        let path = self.root.join("index.sqlite");
        let data = canonical_json(&self.state)?;
        let result = xcs_platform::sqlite::block_on_sqlite_connection(async {
            let mut c = xcs_platform::sqlite::open_index_connection(&path, true, None)
                .await
                .map_err(|e| e.to_string())?;
            c.execute("CREATE TABLE IF NOT EXISTS current_state(singleton INTEGER PRIMARY KEY CHECK(singleton=1),sequence TEXT NOT NULL,snapshot BLOB NOT NULL);CREATE TABLE IF NOT EXISTS entities(kind TEXT NOT NULL,id TEXT NOT NULL,json BLOB NOT NULL,PRIMARY KEY(kind,id));").await.map_err(|e|e.to_string())?;
            let mut tx = c.begin().await.map_err(|e| e.to_string())?;
            sqlx::query("INSERT INTO current_state VALUES(1,?,?) ON CONFLICT(singleton) DO UPDATE SET sequence=excluded.sequence,snapshot=excluded.snapshot").bind(self.state.sequence.to_string()).bind(data).execute(&mut *tx).await.map_err(|e|e.to_string())?;
            sqlx::query("DELETE FROM entities")
                .execute(&mut *tx)
                .await
                .map_err(|e| e.to_string())?;
            let v = serde_json::to_value(&self.state).map_err(|e| e.to_string())?;
            for (kind, map) in v.as_object().expect("state map") {
                if let Some(map) = map.as_object() {
                    for (id, value) in map {
                        sqlx::query("INSERT INTO entities(kind,id,json) VALUES(?,?,?)")
                            .bind(kind)
                            .bind(id)
                            .bind(serde_json::to_vec(value).map_err(|e| e.to_string())?)
                            .execute(&mut *tx)
                            .await
                            .map_err(|e| e.to_string())?;
                    }
                }
            }
            tx.commit().await.map_err(|e| e.to_string())?;
            c.close().await.map_err(|e| e.to_string())?;
            Ok::<(), String>(())
        });
        result.map_err(|e| Error::new("INDEX_REBUILD_FAILED", e))
    }
}
