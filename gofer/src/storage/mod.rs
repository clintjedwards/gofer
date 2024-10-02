//! Contains the data storage interface in which Gofer stores all internal data.
//!
//! As a special concession made we use String to keep epoch millisecond time due to Sqlite's limitation
//! in using only i64. We want most epoch millisecond representations to instead just be u64
//!
//! ## Transactions
//!
//! Transactions are handled by calling `open_tx` like so:
//!
//! ```ignore
//! let mut tx = storage.open_tx().await;
//! let some_db_call(&mut tx).await;
//! let some_other_db_call(&mut tx).await;
//! tx.commit() // Make sure you call commit or changes made inside the transaction wont be changed.
//! ```
//! Sqlite optimizations started from: https://kerkour.com/sqlite-for-servers
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
use sqlx::{
    migrate, pool::PoolConnection, sqlite::SqliteConnectOptions, sqlite::SqlitePoolOptions, Pool,
    Sqlite, Transaction,
};
use std::str::FromStr;
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
    write_pool: Pool<Sqlite>,
    read_pool: Pool<Sqlite>,
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

        // We create two different pools of connections. The read pool has many connections and is high concurrency.
        // The write pool is essentially a single connection in which only one write can be made at a time.
        // Not using this paradigm may result in sqlite "database is locked(error: 5)" errors because of the
        // manner in which sqlite handles transactions.
        let connect_options = SqliteConnectOptions::from_str(&format!("sqlite://{}", path))
            .unwrap()
            // * journal_mode: Turns on WAL mode which increases concurrency and reliability.
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
            // * synchronous: Tells sqlite to not sync to disk as often and specifically only focus on syncing at critcal
            //   junctures. This makes sqlite speedier and also has no downside because we have WAL mode.
            .synchronous(sqlx::sqlite::SqliteSynchronous::Normal)
            // * foreign_keys: Turns on relational style foreign keys. A must have.
            .foreign_keys(true)
            // * busy_timeout: How long should a sqlite query try before it returns an error. Very helpful to avoid
            .busy_timeout(std::time::Duration::from_secs(5));

        let read_pool = SqlitePoolOptions::new()
            .max_connections(10)
            .connect_with(connect_options.clone())
            .await?;
        let write_pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(connect_options)
            .await?;

        migrate!("src/storage/migrations")
            .run(&write_pool)
            .await
            .unwrap();

        Ok(Db {
            write_pool,
            read_pool,
        })
    }

    pub async fn write_conn(&self) -> Result<PoolConnection<Sqlite>, StorageError> {
        self.write_pool
            .acquire()
            .await
            .map_err(|e| StorageError::Connection(format!("{:?}", e)))
    }

    pub async fn read_conn(&self) -> Result<PoolConnection<Sqlite>, StorageError> {
        self.read_pool
            .acquire()
            .await
            .map_err(|e| StorageError::Connection(format!("{:?}", e)))
    }

    pub async fn open_tx(&self) -> Result<Transaction<'_, Sqlite>, StorageError> {
        let mut tx = self
            .write_pool
            .begin()
            .await
            .map_err(|e| StorageError::Connection(format!("{:?}", e)))?;

        // This is a hack to support IMMEDIATE transaction locks within sqlite.
        //
        // Sqlite by default opens all transactions as deferred, this means that the transaction is registered, but
        // no locks are held until a write operation comes in. The downside to this is that if during the connection
        // there is a write to the database the entire transaction is void (and the error returned is "Sqlite DB busy").
        // To overcome this usually you can set the transaction to instead be IMMEDIATE, which would establish a lock
        // that would then prevent write access to the database until the transaction was finished.
        //
        // The further issue is that the sqlx library does not yet support opening transactions in IMMEDIATE mode.
        // To subvert that, we instead force a write operation to a dummy table to force the transaction to grab a lock
        // before it can be preempted by some other write.
        //
        // Relevant ticket here: https://github.com/launchbadge/sqlx/issues/481
        sqlx::query("INSERT INTO transaction_mutex (id, lock) VALUES (1, 1);")
            .execute(tx.as_mut())
            .await
            .map_err(|e| {
                StorageError::Connection(format!(
                "Error while attempting to start transaction using transaction_mutex table; {:?}",
                e
            ))
            })?;

        Ok(tx)
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
