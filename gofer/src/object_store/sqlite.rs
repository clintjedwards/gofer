use super::{ObjectStore, ObjectStoreError, Value};
use anyhow::Result;
use async_trait::async_trait;
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::{Connection, Transaction};
use serde::Deserialize;
use std::{fs::File, io, ops::Deref, path::Path};

#[derive(Deserialize, Default, Debug, Clone)]
pub struct Config {
    pub path: String,
}

#[derive(Debug, Clone)]
pub struct Engine {
    read_pool: Pool<SqliteConnectionManager>,
    write_pool: Pool<SqliteConnectionManager>,
}

/// Rusqlite Errors are determined by database error code. We map these to the specific code so that
/// when we come back with a database error we can detect which one happened.
/// See the codes here: https://www.sqlite.org/rescode.html
fn map_rusqlite_error(e: rusqlite::Error, query: &str) -> ObjectStoreError {
    match e {
        rusqlite::Error::QueryReturnedNoRows => ObjectStoreError::NotFound,
        rusqlite::Error::SqliteFailure(error, err_str) => {
            if let Some(err_code) = error.code() {
                match err_code.deref() {
                    "1555" => ObjectStoreError::Exists,
                    "787" => ObjectStoreError::ForeignKeyViolation(err_str.to_string()),
                    _ => ObjectStoreError::GenericDBError {
                        code: Some(err_code.to_string()),
                        message: format!("Unmapped error occurred; {}", err_str),
                        query: query.into(),
                    },
                }
            } else {
                ObjectStoreError::GenericDBError {
                    code: None,
                    message: err_str.to_string(),
                    query: query.into(),
                }
            }
        }
        _ => ObjectStoreError::GenericDBError {
            code: None,
            message: e.to_string(),
            query: query.into(),
        },
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

        let read_manager =
            r2d2_sqlite::SqliteConnectionManager::file(&config.path).with_init(|conn| {
                // The PRAGMA settings here control various sqlite settings that are required for a working and performant
                // sqlite database tuned for a highly concurrent web server. In order:
                // * journal_mode: Turns on WAL mode which increases concurrency and reliability.
                // * synchronous: Tells sqlite to not sync to disk as often and specifically only focus on syncing at critcal
                //   junctures. This makes sqlite speedier and also has no downside because we have WAL mode.
                // * foreign_keys: Turns on relational style foreign keys. A must have.
                // * busy_timeout: How long should a sqlite query try before it returns an error. Very helpful to avoid
                // * sqlite "database busy/database is locked" errors.
                // * cache_size(-1048576): Refers to the amount of memory sqlite will use as a cache. Obviously more memory
                //    is good. The negative sign means use KB, the value is in Kilobytes. In total it means use 1GB of memory.
                // * temp_store: Tells sqlite to store temporary objects in memory rather than disk.
                conn.execute_batch(
                    "PRAGMA journal_mode = WAL;
                 PRAGMA synchronous = NORMAL;
                 PRAGMA foreign_keys = ON;
                 PRAGMA busy_timeout = 5000;
                 PRAGMA cache_size = -1048576;
                 PRAGMA temp_store = MEMORY;",
                )
            });
        let write_manager =
            r2d2_sqlite::SqliteConnectionManager::file(&config.path).with_init(|conn| {
                conn.execute_batch(
                    "PRAGMA journal_mode = WAL;
                 PRAGMA synchronous = NORMAL;
                 PRAGMA foreign_keys = ON;
                 PRAGMA busy_timeout = 5000;
                 PRAGMA cache_size = -1048576;
                 PRAGMA temp_store = MEMORY;",
                )
            });

        // We create two different pools of connections. The read pool has many connections and is high concurrency.
        // The write pool is essentially a single connection in which only one write can be made at a time.
        // Not using this paradigm may result in sqlite "database is locked(error: 5)" errors because of the
        // manner in which sqlite handles transactions.
        let read_pool = r2d2::Pool::builder().build(read_manager).unwrap();
        let write_pool = r2d2::Pool::builder()
            .max_size(1)
            .build(write_manager)
            .unwrap();

        let mut conn = write_pool.get().unwrap();
        //TODO: Sqlquery this table

        do something here

        run_migrations(&mut conn).unwrap();

        Engine {
            read_pool,
            write_pool,
        }
    }

    /// Grab a read connection from the pool. Read connections have high concurrency and don't
    /// block other reads or writes from happening.
    pub fn read_conn(&self) -> Result<Connection, ObjectStoreError> {
        self.read_pool
            .get()
            .map_err(|e| ObjectStoreError::Connection(format!("{:?}", e)))
    }

    /// Grab a write connection. Only one write connection is shared as sqlite only supports a single
    /// writer. Attempting to execute a write will hold a global lock and prevent both reads and writes
    /// from happening during that time.
    pub fn write_conn(&self) -> Result<Connection, ObjectStoreError> {
        self.write_pool
            .get()
            .map_err(|e| ObjectStoreError::Connection(format!("{:?}", e)))
    }

    /// We always open transactions with the Immediate type. This causes sqlite to immediately hold a lock for that
    /// transaction, instead of its default behavior which is only to attempt to grab a lock before the first write
    /// call. Declaring that we're in a transaction early prevents sqlite_busy errors by preventing the race condition
    /// where we'll open a transaction, make a bunch of read calls and then finally a write call only to realize that
    /// the underlying data has changed because the transaction was deferred and another writer had it's way with
    /// the database.
    pub fn open_tx<'a>(
        &self,
        conn: &'a mut Connection,
    ) -> Result<Transaction<'a>, ObjectStoreError> {
        let tx = conn
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(|e| {
                ObjectStoreError::Connection(format!("Could not open transaction: {:?}", e))
            });

        tx
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
            .map_err(|e| map_lx_error(e, sql))
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
