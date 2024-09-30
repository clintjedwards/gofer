use super::{ObjectStore, ObjectStoreError, Value};
use anyhow::Result;
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use sea_query::SqliteQueryBuilder;
use sea_query::{ColumnDef, Expr, Iden, Query, Table};
use sea_query_rusqlite::RusqliteBinder;
use serde::Deserialize;
use std::{fs::File, io, path::Path};
use tokio_rusqlite::{Connection, Error, ErrorCode, Transaction, TransactionBehavior};

#[derive(Deserialize, Default, Debug, Clone)]
pub struct Config {
    pub path: String,
}

#[derive(Iden)]
enum ObjectTable {
    Table,
    Key,
    Value,
}

#[derive(Debug, Clone)]
pub struct Engine {
    read_pool: Pool<SqliteConnectionManager>,
    write_pool: Pool<SqliteConnectionManager>,
}

/// Rusqlite Errors are determined by database error code. We map these to the specific code so that
/// when we come back with a database error we can detect which one happened.
/// See the codes here: https://www.sqlite.org/rescode.html
fn map_rusqlite_error(e: Error, query: &str) -> ObjectStoreError {
    match e {
        Error::QueryReturnedNoRows => ObjectStoreError::NotFound,
        Error::SqliteFailure(error, err_str) => match error.code {
            ErrorCode::ConstraintViolation => ObjectStoreError::Exists,
            _ => ObjectStoreError::GenericDBError {
                code: Some(error.to_string()),
                message: format!("Unmapped error occurred; {}", err_str.unwrap_or_default()),
                query: query.into(),
            },
        },
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
    pub fn new(config: &Config) -> Self {
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

        let conn = write_pool.get().unwrap();

        // Create initial table.
        let create_table_statement = Table::create()
            .table(ObjectTable::Table)
            .if_not_exists()
            .col(
                ColumnDef::new(ObjectTable::Key)
                    .text()
                    .not_null()
                    .primary_key(),
            )
            .col(ColumnDef::new(ObjectTable::Value).blob().not_null())
            .to_string(SqliteQueryBuilder);

        conn.execute(&create_table_statement, []).unwrap();

        Engine {
            read_pool,
            write_pool,
        }
    }

    /// Grab a read connection from the pool. Read connections have high concurrency and don't
    /// block other reads or writes from happening.
    pub fn read_conn(&self) -> Result<Connection, ObjectStoreError> {
        let conn = self
            .read_pool
            .get()
            .map_err(|e| ObjectStoreError::Connection(format!("{:?}", e)))?;

        Ok(conn)
    }

    /// Grab a write connection. Only one write connection is shared as sqlite only supports a single
    /// writer. Attempting to execute a write will hold a global lock and prevent both reads and writes
    /// from happening during that time.
    pub fn write_conn(&self) -> Result<Connection, ObjectStoreError> {
        let conn = self
            .write_pool
            .get()
            .map_err(|e| ObjectStoreError::Connection(format!("{:?}", e)))?;

        Ok(conn)
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
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|e| {
                ObjectStoreError::Connection(format!("Could not open transaction: {:?}", e))
            });

        tx
    }
}

impl ObjectStore for Engine {
    fn get(&self, key: &str) -> Result<Value, ObjectStoreError> {
        let mut conn = self.read_conn()?;

        let (sql, values) = Query::select()
            .column(ObjectTable::Value)
            .from(ObjectTable::Table)
            .and_where(Expr::col(ObjectTable::Key).eq(key))
            .limit(1)
            .build_rusqlite(SqliteQueryBuilder);

        let mut statement = conn
            .prepare(sql.as_str())
            .map_err(|e| map_rusqlite_error(e, &sql))?;

        let mut rows = statement
            .query(&*values.as_params())
            .map_err(|e| map_rusqlite_error(e, &sql))?;

        if let Some(row) = rows.next().map_err(|e| map_rusqlite_error(e, &sql))? {
            let value: Vec<u8> = row.get_unwrap("value");
            Ok(Value(value))
        } else {
            Err(ObjectStoreError::NotFound)
        }
    }

    fn put(&self, key: &str, content: Vec<u8>, force: bool) -> Result<(), ObjectStoreError> {
        let mut conn = self.write_conn()?;

        let (sql, values) = Query::insert()
            .into_table(ObjectTable::Table)
            .columns([ObjectTable::Key, ObjectTable::Value])
            .values_panic([key.into(), content.clone().into()])
            .build_rusqlite(SqliteQueryBuilder);

        let insert_result = conn.execute(sql.as_str(), &*values.as_params());

        if let Err(e) = insert_result {
            if let Error::SqliteFailure(err, err_str) = e {
                if err.code == ErrorCode::ConstraintViolation {
                    if force {
                        let (update_sql, update_values) = Query::update()
                            .table(ObjectTable::Table)
                            .value(ObjectTable::Value, content)
                            .and_where(Expr::col(ObjectTable::Key).eq(key))
                            .build_rusqlite(SqliteQueryBuilder);

                        conn.execute(update_sql.as_str(), &*update_values.as_params())
                            .map_err(|err| map_rusqlite_error(err, &update_sql))?;
                    } else {
                        return Err(map_rusqlite_error(e, &sql));
                    }
                } else {
                    return Err(map_rusqlite_error(e, &sql));
                }
            } else {
                return Err(map_rusqlite_error(e, &sql));
            }
        }

        Ok(())
    }

    fn list_keys(&self, prefix: &str) -> Result<Vec<String>, ObjectStoreError> {
        let mut conn = self.read_conn()?;

        let (sql, values) = Query::select()
            .column(ObjectTable::Key)
            .from(ObjectTable::Table)
            .and_where(Expr::col(ObjectTable::Key).like(format!("{}%", prefix)))
            .build_rusqlite(SqliteQueryBuilder);

        let mut statement = conn
            .prepare(sql.as_str())
            .map_err(|e| map_rusqlite_error(e, &sql))?;

        let mut rows = statement
            .query(&*values.as_params())
            .map_err(|e| map_rusqlite_error(e, &sql))?;

        let mut keys: Vec<String> = vec![];

        while let Some(row) = rows.next().map_err(|e| map_rusqlite_error(e, &sql))? {
            keys.push(row.get_unwrap("key"));
        }

        Ok(keys)
    }

    fn delete(&self, key: &str) -> Result<(), ObjectStoreError> {
        let mut conn = self.write_conn()?;

        let (sql, values) = Query::delete()
            .from_table(ObjectTable::Table)
            .and_where(Expr::col(ObjectTable::Key).eq(key))
            .build_rusqlite(SqliteQueryBuilder);

        conn.execute(sql.as_str(), &*values.as_params())
            .map(|_| ())
            .map_err(|e| map_rusqlite_error(e, &sql))
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
        pub fn new() -> Self {
            let mut rng = rand::thread_rng();
            let append_num: u16 = rng.gen();
            let storage_path = format!("/tmp/gofer_tests_object_store{}.db", append_num);

            let db = Engine::new(&Config {
                path: storage_path.clone(),
            });

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

    fn setup() -> Result<TestHarness, Box<dyn std::error::Error>> {
        let harness = TestHarness::new();

        let test_key = "test_key";
        let test_value = "test_value".as_bytes();

        harness.db.put(test_key, test_value.to_vec(), false)?;

        Ok(harness)
    }

    /// Basic CRUD can be accomplished.
    fn crud() {
        let harness = setup().unwrap();

        let test_key = "test_key";
        let test_value = Value("test_value".as_bytes().to_vec());

        let returned_value = harness.get(test_key).unwrap();
        assert_eq!(test_value, returned_value);

        let test_value_2 = Value("test_value_2".as_bytes().to_vec());

        harness
            .db
            .put(test_key, test_value_2.clone().0, true)
            .unwrap();

        let returned_value = harness.get(test_key).unwrap();
        assert_eq!(test_value_2, returned_value);

        harness.delete(test_key).unwrap();

        let returned_err = harness.get(test_key).unwrap_err();
        assert_eq!(super::ObjectStoreError::NotFound, returned_err);
    }
}
