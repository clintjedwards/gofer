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
pub mod deployments;
pub mod events;
pub mod extension_registrations;
pub mod extension_subscriptions;
pub mod namespaces;
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
use sqlx::{migrate, pool::PoolConnection, Pool, Sqlite, SqlitePool};
use std::time::{SystemTime, UNIX_EPOCH};
use std::{fs::File, io, ops::Deref, path::Path};

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
    pool: Pool<Sqlite>,
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
        touch_file(Path::new(path)).unwrap();

        let connection_pool = SqlitePool::connect(&format!("file:{}", path))
            .await
            .unwrap();

        // Setting PRAGMAs
        sqlx::query("PRAGMA journal_mode = WAL;")
            .execute(&connection_pool)
            .await?;

        sqlx::query("PRAGMA busy_timeout = 5000;")
            .execute(&connection_pool)
            .await?;

        sqlx::query("PRAGMA foreign_keys = ON;")
            .execute(&connection_pool)
            .await?;

        sqlx::query("PRAGMA strict = ON;")
            .execute(&connection_pool)
            .await?;

        migrate!("src/storage/migrations")
            .run(&connection_pool)
            .await
            .unwrap();

        Ok(Db {
            pool: connection_pool,
        })
    }

    pub async fn conn(&self) -> Result<PoolConnection<Sqlite>, StorageError> {
        self.pool
            .acquire()
            .await
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
