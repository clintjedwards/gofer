use super::*;
use async_trait::async_trait;
use std::{fs::create_dir_all, io, path::Path};

#[derive(Debug, Clone)]
pub struct Engine {
    db: sled::Db,
}

// Create folder if not exists.
fn touch_folder(path: &Path) -> io::Result<()> {
    if !path.exists() {
        create_dir_all(path)?;
    }

    Ok(())
}

impl Engine {
    pub async fn new(path: &str) -> Result<Self, ObjectStoreError> {
        touch_folder(Path::new(path)).unwrap();

        Ok(Self {
            db: sled::open(path).expect("could not open object database"),
        })
    }
}

#[async_trait]
impl Store for Engine {
    async fn get_object(&self, key: &str) -> Result<Vec<u8>, ObjectStoreError> {
        let value = self
            .db
            .get(key)
            .map_err(|e| ObjectStoreError::Unknown(e.to_string()))?;

        if value.is_none() {
            return Err(ObjectStoreError::NotFound);
        };

        Ok(value.unwrap().to_vec())
    }

    async fn put_object(
        &self,
        key: &str,
        value: Vec<u8>,
        force: bool,
    ) -> Result<(), ObjectStoreError> {
        if key.is_empty() {
            return Err(ObjectStoreError::FailedPrecondition);
        };

        if force {
            self.db
                .insert(key, value)
                .map_err(|e| ObjectStoreError::Unknown(e.to_string()))?;
            return Ok(());
        }

        self.db.compare_and_swap::<_, Vec<u8>, _>(key, None, Some(value)).
            map_err(|e| ObjectStoreError::Unknown(e.to_string()))?.
            // If we've reached an error at this level it can only be because the value exists.
            map_err(|_| ObjectStoreError::Exists)?;

        Ok(())
    }

    async fn delete_object(&self, key: &str) -> Result<(), ObjectStoreError> {
        self.db
            .remove(key)
            .map_err(|e| ObjectStoreError::Unknown(e.to_string()))?;

        Ok(())
    }
}
