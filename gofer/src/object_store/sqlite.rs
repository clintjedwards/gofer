use super::{ObjectStore, ObjectStoreError, Value};
use anyhow::Result;
use async_trait::async_trait;
use futures::TryFutureExt;
use serde::Deserialize;
use sqlx::{pool::PoolConnection, Execute, Pool, Sqlite, SqlitePool};
use std::{fs::File, io, ops::Deref, path::Path};

#[derive(Deserialize, Default, Debug, Clone)]
pub struct Config {
    pub path: String,
}

#[derive(Debug, Clone)]
pub struct Engine(Pool<Sqlite>);

/// Sqlite Errors are determined by database error code. We map these to the specific code so that
/// when we come back with a database error we can detect which one happened.
/// See the codes here: https://www.sqlite.org/rescode.html
fn map_sqlx_error(e: sqlx::Error, query: &str) -> ObjectStoreError {
    match e {
        sqlx::Error::RowNotFound => ObjectStoreError::NotFound,
        sqlx::Error::Database(database_err) => {
            if let Some(err_code) = database_err.code() {
                match err_code.deref() {
                    "1555" => ObjectStoreError::Exists,
                    _ => ObjectStoreError::Internal(format!("Error occurred while running object store query; [{err_code}] {database_err}; query: {query}")),
                }
            } else {
                ObjectStoreError::Internal(format!(
                    "Error occurred while running object store query; {database_err}; query: {query}"
                ))
            }
        }
        _ => ObjectStoreError::Internal(format!(
            "Error occurred while running query; {:#?}; query: {query}",
            e
        )),
    }
}

// Create file if not exists.
fn touch_file(path: &Path) -> io::Result<()> {
    if !path.exists() {
        File::create(path)?;
    }

    Ok(())
}

impl Engine {
    pub async fn new(config: &Config) -> Self {
        let config = config.clone();

        touch_file(Path::new(&config.path)).unwrap();

        let connection_pool = SqlitePool::connect(&format!("file:{}", &config.path))
            .await
            .unwrap();

        // Setting PRAGMAs
        sqlx::query("PRAGMA journal_mode = WAL;")
            .execute(&connection_pool)
            .await
            .unwrap();

        sqlx::query("PRAGMA busy_timeout = 5000;")
            .execute(&connection_pool)
            .await
            .unwrap();

        sqlx::query("PRAGMA foreign_keys = ON;")
            .execute(&connection_pool)
            .await
            .unwrap();

        sqlx::query("PRAGMA strict = ON;")
            .execute(&connection_pool)
            .await
            .unwrap();

        sqlx::query(
            r#"CREATE TABLE IF NOT EXISTS objects (
            key   TEXT NOT NULL,
            value BLOB NOT NULL,
            PRIMARY KEY (key)
        ) STRICT;"#,
        )
        .execute(&connection_pool)
        .await
        .unwrap();

        Engine(connection_pool)
    }

    pub async fn conn(&self) -> Result<PoolConnection<Sqlite>, ObjectStoreError> {
        self.0.acquire().await.map_err(|e| {
            ObjectStoreError::Connection(format!(
                "Could not establish connection to object store {:?}",
                e
            ))
        })
    }
}

#[async_trait]
impl ObjectStore for Engine {
    async fn get(&self, key: &str) -> Result<Value, ObjectStoreError> {
        let mut conn = self.conn().await?;

        let query = sqlx::query_as("SELECT value FROM objects WHERE key = ?;").bind(key);

        let sql = query.sql();

        query
            .fetch_one(&mut *conn)
            .map_err(|e| map_sqlx_error(e, sql))
            .await
    }

    async fn put(&self, key: &str, content: Vec<u8>, force: bool) -> Result<(), ObjectStoreError> {
        let mut conn = self.conn().await?;

        let query = sqlx::query("INSERT INTO objects (key, value) VALUES (?, ?);")
            .bind(key)
            .bind(content.clone());

        let sql = query.sql();

        // If there is already a key we provide the functionality to update that key instead of passing back up
        // the conflict error.
        if let Err(e) = query.execute(&mut *conn).await {
            match e {
                sqlx::Error::Database(ref database_err) => {
                    if let Some(err_code) = database_err.code() {
                        match err_code.deref() {
                            "1555" => {
                                if force {
                                    let update_query =
                                        sqlx::query("UPDATE objects SET value = ? WHERE key = ?")
                                            .bind(content)
                                            .bind(key);

                                    let update_sql = update_query.sql();

                                    update_query
                                        .execute(&mut *conn)
                                        .await
                                        .map_err(|err| map_sqlx_error(err, update_sql))?;
                                } else {
                                    return Err(map_sqlx_error(e, sql));
                                };
                            }
                            _ => return Err(map_sqlx_error(e, sql)),
                        }
                    } else {
                        return Err(map_sqlx_error(e, sql));
                    }
                }
                _ => return Err(map_sqlx_error(e, sql)),
            };
        };

        Ok(())
    }

    async fn list_keys(&self, prefix: &str) -> Result<Vec<String>, ObjectStoreError> {
        let mut conn = self.conn().await?;

        let query = sqlx::query_as::<_, (String,)>("SELECT key FROM objects WHERE key LIKE ?%;")
            .bind(prefix);

        let sql = query.sql();

        let rows = query
            .fetch_all(&mut *conn)
            .map_err(|e| map_sqlx_error(e, sql))
            .await?;

        let keys = rows.into_iter().map(|(key,)| key).collect();

        Ok(keys)
    }

    async fn delete(&self, key: &str) -> Result<(), ObjectStoreError> {
        let mut conn = self.conn().await?;

        let query = sqlx::query("DELETE FROM objects WHERE key = ?;").bind(key);

        let sql = query.sql();

        query
            .execute(&mut *conn)
            .map_ok(|_| ())
            .map_err(|e| map_sqlx_error(e, sql))
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::prelude::*;
    use std::ops::Deref;

    pub struct TestHarness {
        pub db: Engine,
        pub storage_path: String,
    }

    impl TestHarness {
        pub async fn new() -> Self {
            let mut rng = rand::thread_rng();
            let append_num: u16 = rng.gen();
            let storage_path = format!("/tmp/gofer_tests_object_store{}.db", append_num);

            let db = Engine::new(&Config {
                path: storage_path.clone(),
            })
            .await;

            Self { db, storage_path }
        }
    }

    impl Deref for TestHarness {
        type Target = Engine;

        fn deref(&self) -> &Self::Target {
            &self.db
        }
    }

    impl Drop for TestHarness {
        fn drop(&mut self) {
            std::fs::remove_file(&self.storage_path).unwrap();
            std::fs::remove_file(format!("{}{}", &self.storage_path, "-shm")).unwrap();
            std::fs::remove_file(format!("{}{}", &self.storage_path, "-wal")).unwrap();
        }
    }

    async fn setup() -> Result<TestHarness, Box<dyn std::error::Error>> {
        let harness = TestHarness::new().await;

        let test_key = "test_key";
        let test_value = "test_value".as_bytes();

        harness.db.put(test_key, test_value.to_vec(), false).await?;

        Ok(harness)
    }

    #[tokio::test]
    /// Basic CRUD can be accomplished.
    async fn crud() {
        let harness = setup().await.unwrap();

        let test_key = "test_key";
        let test_value = Value("test_value".as_bytes().to_vec());

        let returned_value = harness.get(test_key).await.unwrap();
        assert_eq!(test_value, returned_value);

        let test_value_2 = Value("test_value_2".as_bytes().to_vec());

        harness
            .db
            .put(test_key, test_value_2.clone().0, true)
            .await
            .unwrap();

        let returned_value = harness.get(test_key).await.unwrap();
        assert_eq!(test_value_2, returned_value);

        harness.delete(test_key).await.unwrap();

        let returned_err = harness.get(test_key).await.unwrap_err();
        assert_eq!(super::ObjectStoreError::NotFound, returned_err);
    }
}
