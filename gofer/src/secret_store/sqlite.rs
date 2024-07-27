use super::{SecretStore, SecretStoreError, Value};
use aes_gcm::{
    aead::{generic_array::GenericArray, Aead},
    Aes256Gcm, KeyInit,
};
use anyhow::{anyhow, bail, Context, Result};
use async_trait::async_trait;
use futures::TryFutureExt;
use rand::{rngs::OsRng, RngCore};
use serde::Deserialize;
use sqlx::{pool::PoolConnection, Execute, Pool, Sqlite, SqlitePool};
use std::{fs::File, io, ops::Deref, path::Path};
use tracing::{error, instrument};

const NONCE_SIZE: usize = 12; // Standard nonce size for AES-GCM

#[derive(Deserialize, Default, Debug, Clone)]
pub struct Config {
    pub path: String,

    /// Must be 32 characters long.
    pub encryption_key: String,
}

#[derive(Debug, Clone)]
pub struct Engine {
    pub pool: Pool<Sqlite>,
    pub encryption_key: String,
}

/// Sqlite Errors are determined by database error code. We map these to the specific code so that
/// when we come back with a database error we can detect which one happened.
/// See the codes here: https://www.sqlite.org/rescode.html
fn map_sqlx_error(e: sqlx::Error, query: &str) -> SecretStoreError {
    match e {
        sqlx::Error::RowNotFound => SecretStoreError::NotFound,
        sqlx::Error::Database(database_err) => {
            if let Some(err_code) = database_err.code() {
                match err_code.deref() {
                    "1555" => SecretStoreError::Exists,
                    _ => SecretStoreError::Internal(format!("Error occurred while running secret store query; [{err_code}] {database_err}; query: {query}")),
                }
            } else {
                SecretStoreError::Internal(format!(
                    "Error occurred while running secret store query; {database_err}; query: {query}"
                ))
            }
        }
        _ => SecretStoreError::Internal(format!(
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
    pub async fn new(config: &Config) -> Result<Self> {
        let config = config.clone();

        if config.encryption_key.len() < 32 {
            bail!("secret_store.sqlite.encryption_key must be at least 32 characters");
        }

        touch_file(Path::new(&config.path)).unwrap();

        let connection_pool = SqlitePool::connect(&format!("file:{}", &config.path))
            .await
            .context("Could not open Sqlite database")?;

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

        sqlx::query(
            r#"CREATE TABLE IF NOT EXISTS secrets (
            key   TEXT NOT NULL,
            value BLOB NOT NULL,
            PRIMARY KEY (key)
        ) STRICT;"#,
        )
        .execute(&connection_pool)
        .await
        .context("Could not create schema")?;

        Ok(Engine {
            pool: connection_pool,
            encryption_key: config.encryption_key,
        })
    }

    pub async fn conn(&self) -> Result<PoolConnection<Sqlite>, SecretStoreError> {
        self.pool.acquire().await.map_err(|e| {
            SecretStoreError::Connection(format!(
                "Could not establish connection to secret store {:?}",
                e
            ))
        })
    }
}

#[instrument(fields(origin = "secret_store::sqlite"))]
pub fn encrypt(key: &[u8], plaintext: &[u8]) -> Result<Vec<u8>> {
    let cipher = Aes256Gcm::new_from_slice(key)?;

    let mut n = vec![0u8; NONCE_SIZE];
    OsRng.fill_bytes(&mut n);
    let nonce = GenericArray::from_slice(&n);

    let ciphertext = cipher.encrypt(nonce, plaintext.as_ref()).map_err(|e| {
        error!(error = %e, key = String::from_utf8_lossy(key).to_string(), "Could not encrypt value for key");
        anyhow!(
            "Could not encrypt value for key '{}'",
            String::from_utf8_lossy(key)
        )
    })?;
    Ok([nonce.as_slice(), ciphertext.as_slice()].concat())
}

#[instrument(fields(origin = "secret_store::sqlite"))]
pub fn decrypt(key: &[u8], ciphertext: &[u8]) -> Result<Vec<u8>> {
    if ciphertext.len() < 12 {
        bail!("Ciphertext is too short and may be malformed");
    }

    let cipher = Aes256Gcm::new_from_slice(key)?;
    let (nonce, ciphertext) = ciphertext.split_at(NONCE_SIZE);
    let nonce = GenericArray::from_slice(nonce);
    cipher.decrypt(nonce, ciphertext.as_ref()).map_err(|e| {
        error!(error = %e, key = String::from_utf8_lossy(key).to_string(), "Could not decrypt value for key");
        anyhow!(
            "Could not decrypt value for key '{}'",
            String::from_utf8_lossy(key)
        )
    })
}

#[async_trait]
impl SecretStore for Engine {
    async fn get(&self, key: &str) -> Result<Value, SecretStoreError> {
        let mut conn = self.conn().await?;

        let query = sqlx::query_as("SELECT value FROM secrets WHERE key = ?;").bind(key);

        let sql = query.sql();

        let result: Value = query
            .fetch_one(&mut *conn)
            .map_err(|e| map_sqlx_error(e, sql))
            .await?;

        let decrypted_value = decrypt(&self.encryption_key.clone().into_bytes(), &result.0)
            .map_err(|_| {
                SecretStoreError::Internal("Could not decrypt value while getting secret".into())
            })?;

        Ok(Value(decrypted_value))
    }

    async fn put(&self, key: &str, content: Vec<u8>, force: bool) -> Result<(), SecretStoreError> {
        let encrypted_value = encrypt(&self.encryption_key.clone().into_bytes(), &content)
            .map_err(|_| {
                SecretStoreError::Internal("Could not encrypt value while inserting secret".into())
            })?;

        let mut conn = self.conn().await?;

        let query = sqlx::query("INSERT INTO secrets (key, value) VALUES (?, ?);")
            .bind(key)
            .bind(encrypted_value.clone());

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
                                        sqlx::query("UPDATE secrets SET value = ? WHERE key = ?")
                                            .bind(encrypted_value)
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

    async fn list_keys(&self, prefix: &str) -> Result<Vec<String>, SecretStoreError> {
        let mut conn = self.conn().await?;

        let query = sqlx::query_as::<_, (String,)>("SELECT key FROM secrets WHERE key LIKE ?%;")
            .bind(prefix);

        let sql = query.sql();

        let rows = query
            .fetch_all(&mut *conn)
            .map_err(|e| map_sqlx_error(e, sql))
            .await?;

        let keys = rows.into_iter().map(|(key,)| key).collect();

        Ok(keys)
    }

    async fn delete(&self, key: &str) -> Result<(), SecretStoreError> {
        let mut conn = self.conn().await?;

        let query = sqlx::query("DELETE FROM secrets WHERE key = ?;").bind(key);

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
            let storage_path = format!("/tmp/gofer_tests_secret_store{}.db", append_num);

            let db = Engine::new(&Config {
                path: storage_path.clone(),
                encryption_key: "mysuperduperdupersupersecretkey_".into(),
            })
            .await
            .unwrap();

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
        assert_eq!(super::SecretStoreError::NotFound, returned_err);
    }
}
