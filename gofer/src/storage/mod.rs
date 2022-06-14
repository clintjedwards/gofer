mod namespaces;
mod pipelines;
mod runs;
mod task_runs;

#[cfg(test)]
mod tests;

use sqlx::{migrate, Pool, Sqlite, SqlitePool};
use std::{error::Error, fmt, fs::File, io, path::Path};

/// The maximum amount of rows that can be returned by any single query.
const MAX_ROW_LIMIT: u64 = 200;

#[derive(thiserror::Error, Debug, PartialEq, Eq)]
pub enum StorageError {
    #[error("requested entity not found")]
    NotFound,

    #[error("entity already exists")]
    Exists,

    #[error("could not parse value '{value}' for column '{column}' from database; {err}")]
    Parse {
        value: String,
        column: String,
        err: String,
    },

    #[error("entity was not in correct state for db operation")]
    FailedPrecondition,

    #[error("unexpected storage error occurred; {0}")]
    Unknown(String),
}

#[derive(Debug)]
pub enum SqliteErrors {
    Constraint,
}

/// Sqlite Errors are determined by database error code. We map these to the specific code so that
/// when we come back with a database error we can detect which one happened.
/// See the codes here: https://www.sqlite.org/rescode.html
impl SqliteErrors {
    fn value(&self) -> String {
        match *self {
            SqliteErrors::Constraint => "1555".to_string(),
        }
    }
}

impl fmt::Display for SqliteErrors {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
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
    pub async fn new(path: &str) -> Result<Self, Box<dyn Error>> {
        touch_file(Path::new(path)).unwrap();

        let connection_pool = SqlitePool::connect(&format!("file:{}", path))
            .await
            .unwrap();

        migrate!("src/storage/migrations")
            .run(&connection_pool)
            .await
            .unwrap();

        Ok(Db {
            pool: connection_pool,
        })
    }
}
