//! Contains the data storage interface in which Gofer stores all internal data.
//!
//! As a special concession made we use String to keep epoch millisecond time due to Sqlite's limitation
//! in using only i64. We want most epoch millisecond representations to instead just be u64
//!
//! ## Transactions
//!
//! Transactions are handled by calling `begin` on [`Db::conn`] like so:
//!
//! ```ignore
//! let mut tx = match conn.begin().await.unwrap();
//! let some_db_call(&mut tx).await;
//! let some_other_db_call(&mut tx).await;
//! tx.commit() // Make sure you call commit or changes made inside the transaction wont be changed.
//! ```
//! The tx object consumes the conn object preventing any further calls outside the transaction for the scope of
//! tx.
//!
//! Sqlite tuning with help from: https://kerkour.com/sqlite-for-servers
//!
pub mod deployments;
pub mod events;
pub mod extension_registrations;
pub mod extension_subscriptions;
pub mod namespaces;
pub mod object_store_extension_keys;
pub mod object_store_pipeline_keys;
pub mod object_store_run_keys;
pub mod pipeline_configs;
pub mod pipeline_metadata;
pub mod roles;
pub mod runs;
pub mod secret_store_global_keys;
pub mod secret_store_pipeline_keys;
pub mod system;
pub mod task_executions;
pub mod tasks;
pub mod tokens;

use anyhow::Result;
use r2d2::{Pool, PooledConnection};
use r2d2_sqlite::SqliteConnectionManager;
use std::time::{SystemTime, UNIX_EPOCH};
use std::{fs::File, io, path::Path};

#[derive(thiserror::Error, Debug, PartialEq, Eq)]
pub enum StorageError {
    #[error("could not establish connection to database; {0}")]
    Connection(String),

    #[error("requested entity not found")]
    NotFound,

    #[error("entity already exists")]
    Exists,

    #[error("request did not update any fields")]
    NoFieldsUpdated,

    #[error("did not find required foreign key for query; {0}")]
    ForeignKeyViolation(String),

    #[error(
        "unexpected storage error occurred; code: {code:?}; message: {message}; query: {query}"
    )]
    GenericDBError {
        code: Option<String>,
        message: String,
        query: String,
    },
}

/// Sqlite Errors are determined by database error code. We map these to the specific code so that
/// when we come back with a database error we can detect which one happened.
/// See the codes here: https://www.sqlite.org/rescode.html
fn map_sqlx_error(e: sqlx::Error, query: &str) -> StorageError {
    match e {
        sqlx::Error::RowNotFound => StorageError::NotFound,
        sqlx::Error::Database(database_err) => {
            if let Some(err_code) = database_err.code() {
                match err_code.deref() {
                    "1555" => StorageError::Exists,
                    "787" => StorageError::ForeignKeyViolation(database_err.to_string()),
                    _ => StorageError::GenericDBError {
                        code: Some(err_code.to_string()),
                        message: format!("Unmapped error occurred; {}", database_err),
                        query: query.into(),
                    },
                }
            } else {
                StorageError::GenericDBError {
                    code: None,
                    message: database_err.to_string(),
                    query: query.into(),
                }
            }
        }
        _ => StorageError::GenericDBError {
            code: None,
            message: e.to_string(),
            query: query.into(),
        },
    }
}

#[derive(Debug, Clone)]
pub struct Db {
    read_pool: Pool<SqliteConnectionManager>,
    write_pool: Pool<SqliteConnectionManager>,
}

// Create file if not exists.
fn touch_file(path: &Path) -> io::Result<()> {
    if !path.exists() {
        File::create(path)?;
    }

    Ok(())
}

impl Db {
    pub async fn new(path: &str) -> Result<Self> {
        let path = Path::new(path);
        touch_file(&path).unwrap();

        let read_manager = r2d2_sqlite::SqliteConnectionManager::file(&path).with_init(|conn| {
            // The PRAGMA settings here control various sqlite settings that are required for a working and performant
            // sqlite database. In order:
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
        let write_manager = r2d2_sqlite::SqliteConnectionManager::file(&path).with_init(|conn| {
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

        // TODO: Figure out migrations
        // migrate!("src/storage/migrations")
        //     .run(&connection_pool)
        //     .await
        //     .unwrap();

        Ok(Db {
            read_pool,
            write_pool,
        })
    }

    pub fn read_conn(&self) -> Result<PooledConnection<SqliteConnectionManager>, StorageError> {
        self.read_pool
            .get()
            .map_err(|e| StorageError::Connection(format!("{:?}", e)))
    }

    pub fn write_conn(&self) -> Result<PooledConnection<SqliteConnectionManager>, StorageError> {
        self.write_pool
            .get()
            .map_err(|e| StorageError::Connection(format!("{:?}", e)))
    }
}

/// Return the current epoch time in milliseconds.
pub fn epoch_milli() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::prelude::*;
    use std::ops::Deref;

    pub struct TestHarness {
        pub db: Db,
        pub storage_path: String,
    }

    impl TestHarness {
        pub async fn new() -> Self {
            let mut rng = rand::thread_rng();
            let append_num: u16 = rng.gen();
            let storage_path = format!("/tmp/gofer_tests_storage{}.db", append_num);

            let db = Db::new(&storage_path).await.unwrap();

            Self { db, storage_path }
        }
    }

    impl Deref for TestHarness {
        type Target = Db;

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
}
