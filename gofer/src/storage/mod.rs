//! Contains the data storage interface in which Gofer stores all internal data.
//!
//! As a special concession made we use String to keep epoch millisecond time due to Sqlite's limitation
//! in using only i64. We want most epoch millisecond representations to instead just be u64
//!
//! ## Transactions
//!
//! Transactions are handled by calling `open_tx` on [`PooledConnection<SqliteConnectionManager>`] like so:
//!
//! ```ignore
//! let mut conn = db.write_conn();
//! let tx = db.open_tx(&mut conn);
//! tx.commit() // Make sure you call commit or changes made inside the transaction wont be written.
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
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::{Connection, Transaction};
use rust_embed::RustEmbed;
use std::time::{SystemTime, UNIX_EPOCH};
use std::{fs::File, io, path::Path};

#[derive(RustEmbed)]
#[folder = "src/storage/migrations"]
pub struct EmbeddedMigrations;

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

/// In order to make downstream storage functions support taking the possibility of a [`rusqlite::Transaction`]
/// or a [`rusqlite::Connection`] object we must create an interface over the common functions.
pub trait Executable {
    fn exec(&self, query: &str, params: &dyn rusqlite::Params) -> rusqlite::Result<usize>;
    fn prepare(&self, query: &str) -> rusqlite::Result<rusqlite::Statement<'_>>;
}

impl Executable for Connection {
    fn exec(&self, query: &str, params: &dyn rusqlite::Params) -> rusqlite::Result<usize> {
        self.execute(query, params)
    }

    fn prepare(&self, query: &str) -> rusqlite::Result<rusqlite::Statement<'_>> {
        self.prepare(query)
    }
}

impl Executable for Transaction<'_> {
    fn exec(&self, query: &str, params: &dyn rusqlite::Params) -> rusqlite::Result<usize> {
        self.execute(query, params)
    }

    fn prepare(&self, query: &str) -> rusqlite::Result<rusqlite::Statement<'_>> {
        self.prepare(query)
    }
}

/// Rusqlite Errors are determined by database error code. We map these to the specific code so that
/// when we come back with a database error we can detect which one happened.
/// See the codes here: https://www.sqlite.org/rescode.html
fn map_rusqlite_error(e: rusqlite::Error, query: &str) -> StorageError {
    match e {
        rusqlite::Error::QueryReturnedNoRows => StorageError::NotFound,
        rusqlite::Error::SqliteFailure(error, err_str) => {
            if let Some(err_code) = error.code() {
                match err_code.deref() {
                    "1555" => StorageError::Exists,
                    "787" => StorageError::ForeignKeyViolation(err_str.to_string()),
                    _ => StorageError::GenericDBError {
                        code: Some(err_code.to_string()),
                        message: format!("Unmapped error occurred; {}", err_str),
                        query: query.into(),
                    },
                }
            } else {
                StorageError::GenericDBError {
                    code: None,
                    message: err_str.to_string(),
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
    pub fn new(path: &str) -> Result<Self> {
        let path = Path::new(path);
        touch_file(&path).unwrap();

        let read_manager = r2d2_sqlite::SqliteConnectionManager::file(&path).with_init(|conn| {
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

        let mut conn = write_pool.get().unwrap();
        run_migrations(&mut conn).unwrap();

        Ok(Db {
            read_pool,
            write_pool,
        })
    }

    /// Grab a read connection from the pool. Read connections have high concurrency and don't
    /// block other reads or writes from happening.
    pub fn read_conn(&self) -> Result<Connection, StorageError> {
        self.read_pool
            .get()
            .map_err(|e| StorageError::Connection(format!("{:?}", e)))
    }

    /// Grab a write connection. Only one write connection is shared as sqlite only supports a single
    /// writer. Attempting to execute a write will hold a global lock and prevent both reads and writes
    /// from happening during that time.
    pub fn write_conn(&self) -> Result<Connection, StorageError> {
        self.write_pool
            .get()
            .map_err(|e| StorageError::Connection(format!("{:?}", e)))
    }

    /// We always open transactions with the Immediate type. This causes sqlite to immediately hold a lock for that
    /// transaction, instead of its default behavior which is only to attempt to grab a lock before the first write
    /// call. Declaring that we're in a transaction early prevents sqlite_busy errors by preventing the race condition
    /// where we'll open a transaction, make a bunch of read calls and then finally a write call only to realize that
    /// the underlying data has changed because the transaction was deferred and another writer had it's way with
    /// the database.
    pub fn open_tx<'a>(&self, conn: &'a mut Connection) -> Transaction<'a> {
        let tx = conn
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .unwrap();

        tx
    }
}

fn run_migrations(conn: &Connection) -> Result<()> {
    let tx = conn.transaction()?;

    for migration in EmbeddedMigrations::iter() {
        let sql_content = std::fs::read_to_string(migration).expect("Failed to read the .sql file");
        tx.execute_batch(&sql_content)?;
    }

    tx.commit()?;

    Ok(())
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
        pub fn new() -> Self {
            let mut rng = rand::thread_rng();
            let append_num: u16 = rng.gen();
            let storage_path = format!("/tmp/gofer_tests_storage{}.db", append_num);

            let db = Db::new(&storage_path).unwrap();

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
