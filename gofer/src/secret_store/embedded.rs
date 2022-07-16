use super::*;
use async_trait::async_trait;
use orion::aead;
use std::{fs::create_dir_all, io, path::Path};

#[derive(Debug, Clone)]
pub struct Engine {
    db: sled::Db,
    encryption_key: String,
}

// Create folder if not exists.
fn touch_folder(path: &Path) -> io::Result<()> {
    if !path.exists() {
        create_dir_all(path)?;
    }

    Ok(())
}

impl Engine {
    pub async fn new(path: &str, encryption_key: &str) -> Result<Self, SecretStoreError> {
        touch_folder(Path::new(path)).unwrap();

        if encryption_key.len() != 32 {
            return Err(SecretStoreError::FailedInitPrecondition(
                "encryption key length must be 32 characters".to_string(),
            ));
        }

        Ok(Self {
            db: sled::open(path).expect("could not open object database"),
            encryption_key: encryption_key.to_string(),
        })
    }

    /// Used to encrypt the plaintext secret before storing. The key provided
    /// MUST be 32 characters.
    fn encrypt(&self, plaintext: &str) -> Result<Vec<u8>, SecretStoreError> {
        let key = aead::SecretKey::from_slice(self.encryption_key.as_bytes())
            .map_err(|e| SecretStoreError::FailedEncryption(format!("{:?}", e)))?;

        let cipher_text = aead::seal(&key, plaintext.as_bytes())
            .map_err(|e| SecretStoreError::FailedEncryption(format!("{:?}", e)))?;

        Ok(cipher_text)
    }

    /// Used to decrypt the encrypted secret before passing back to user. The key
    /// provided MUST be 32 characters.
    fn decrypt(&self, cipher_text: Vec<u8>) -> Result<Vec<u8>, SecretStoreError> {
        let key = aead::SecretKey::from_slice(self.encryption_key.as_bytes())
            .map_err(|e| SecretStoreError::FailedEncryption(format!("{:?}", e)))?;

        let decryption_text = aead::open(&key, &cipher_text)
            .map_err(|e| SecretStoreError::FailedEncryption(format!("{:?}", e)))?;

        Ok(decryption_text)
    }
}

#[async_trait]
impl Store for Engine {
    async fn get_secret(&self, key: &str) -> Result<Vec<u8>, SecretStoreError> {
        let value = self
            .db
            .get(key)
            .map_err(|e| SecretStoreError::Unknown(e.to_string()))?;

        if value.is_none() {
            return Err(SecretStoreError::NotFound);
        };

        let secret = self.decrypt(value.unwrap().to_vec())?;

        Ok(secret)
    }

    async fn put_secret(
        &self,
        key: &str,
        value: &str,
        force: bool,
    ) -> Result<(), SecretStoreError> {
        if key.is_empty() {
            return Err(SecretStoreError::FailedPrecondition);
        };

        let secret = self.encrypt(value).unwrap();

        if force {
            self.db
                .insert(key, secret)
                .map_err(|e| SecretStoreError::Unknown(e.to_string()))?;
            return Ok(());
        }

        self.db.compare_and_swap::<_, Vec<u8>, _>(key, None, Some(secret)).
            map_err(|e| SecretStoreError::Unknown(e.to_string()))?.
            // If we've reached an error at this level it can only be because the value exists.
            map_err(|_| SecretStoreError::Exists)?;

        Ok(())
    }

    async fn delete_secret(&self, key: &str) -> Result<(), SecretStoreError> {
        self.db
            .remove(key)
            .map_err(|e| SecretStoreError::Unknown(e.to_string()))?;

        Ok(())
    }
}
