use super::{ObjectStore, ObjectStoreError};
use anyhow::Result;
use async_trait::async_trait;
use bytes::Bytes;
use futures::StreamExt;
use object_store::{local::LocalFileSystem, ObjectStore as ObjStore, WriteMultipart};
use serde::Deserialize;
use std::pin::Pin;
use tokio_stream::Stream;

impl From<object_store::Error> for ObjectStoreError {
    fn from(err: object_store::Error) -> Self {
        match err {
            object_store::Error::NotFound { .. } => ObjectStoreError::NotFound,
            _ => ObjectStoreError::Internal("Unexpected error occurred".into()),
        }
    }
}

#[derive(Deserialize, Default, Debug, Clone)]
pub struct Config {
    /// The path of the directory that should contain the object files.
    pub path: String,
}

#[derive(Debug)]
pub struct Engine(LocalFileSystem);

impl Engine {
    pub async fn new(config: &Config) -> Self {
        let config = config.clone();

        std::fs::create_dir_all(&config.path).unwrap();

        let store = LocalFileSystem::new_with_prefix(config.path).unwrap();

        Engine(store)
    }
}

#[async_trait]
impl ObjectStore for Engine {
    async fn exists(&self, key: &str) -> Result<bool, ObjectStoreError> {
        let path = object_store::path::Path::from(key);

        match self.0.head(&path).await {
            Ok(_) => Ok(true),
            Err(e) => {
                if let object_store::Error::NotFound { path: _, source: _ } = e {
                    Ok(false)
                } else {
                    Err(ObjectStoreError::from(e))
                }
            }
        }
    }

    async fn get(&self, key: &str) -> Result<Bytes, ObjectStoreError> {
        let path = object_store::path::Path::from(key);

        let result = self.0.get(&path).await.map_err(ObjectStoreError::from)?;

        let object = result.bytes().await.map_err(ObjectStoreError::from)?;

        Ok(object)
    }

    async fn get_stream(
        &self,
        key: &str,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<Bytes, ObjectStoreError>> + Send>>, ObjectStoreError>
    {
        let path = object_store::path::Path::from(key);

        let object_stream = self
            .0
            .get(&path)
            .await
            .map_err(ObjectStoreError::from)?
            .into_stream();

        let object_stream = object_stream.map(|item| item.map_err(ObjectStoreError::from));

        Ok(Box::pin(object_stream))
    }

    async fn put(&self, key: &str, content: Bytes, force: bool) -> Result<(), ObjectStoreError> {
        let path = object_store::path::Path::from(key);

        let meta = self.0.head(&path).await;

        // We've found an object, but the user did not pass force, return an error.
        if meta.is_ok() && !force {
            return Err(ObjectStoreError::Exists);
        }

        let payload = object_store::PutPayload::from_bytes(content);

        self.0
            .put(&path, payload)
            .await
            .map_err(ObjectStoreError::from)?;

        Ok(())
    }

    // This function should probably have some clean up in case we don't fully successfully write the
    // file in it's entirety.
    async fn put_stream(
        &self,
        key: &str,
        mut content: Pin<Box<dyn Stream<Item = Bytes> + Send>>,
    ) -> Result<(), ObjectStoreError> {
        let path = object_store::path::Path::from(key);

        let upload = self
            .0
            .put_multipart(&path)
            .await
            .map_err(ObjectStoreError::from)?;

        let mut writer = WriteMultipart::new(upload);

        // Read from the user stream chunk by chunk
        while let Some(chunk) = content.next().await {
            writer.write(&chunk);
        }

        writer.finish().await.map_err(ObjectStoreError::from)?;

        Ok(())
    }

    async fn delete(&self, key: &str) -> Result<(), ObjectStoreError> {
        let path = object_store::path::Path::from(key);

        self.0.delete(&path).await.map_err(ObjectStoreError::from)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::{BufMut, BytesMut};
    use futures::stream;
    use rand::prelude::*;
    use std::ops::Deref;
    use tokio_stream::StreamExt;

    pub struct TestHarness {
        pub db: Engine,
        pub storage_path: String,
    }

    impl TestHarness {
        pub async fn new() -> Self {
            let mut rng = rand::thread_rng();
            let append_num: u16 = rng.gen();
            let storage_path = format!("/tmp/gofer_tests_object_store_{}", append_num);

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
            std::fs::remove_dir_all(&self.storage_path).unwrap();
        }
    }

    async fn setup() -> Result<TestHarness, Box<dyn std::error::Error>> {
        let harness = TestHarness::new().await;

        let test_key = "test_key";
        let test_value = Bytes::from("test_value");

        harness.db.put(test_key, test_value, false).await?;

        let test_key = "test_key_stream";
        let test_value = Bytes::from("test_value_stream");
        let test_value_stream = stream::once(async move { test_value });
        let test_value_stream: Pin<Box<dyn Stream<Item = Bytes> + Send>> =
            Box::pin(test_value_stream);

        harness.db.put_stream(test_key, test_value_stream).await?;

        Ok(harness)
    }

    #[tokio::test]
    /// Basic CRUD can be accomplished.
    async fn crud() {
        let harness = setup().await.unwrap();

        let test_key = "test_key";
        let test_value = Bytes::from("test_value");

        let returned_value = harness.get(test_key).await.unwrap();
        assert_eq!(test_value, returned_value);

        let test_value_2 = Bytes::from("test_value_2");

        harness
            .db
            .put(test_key, test_value_2.clone(), true)
            .await
            .unwrap();

        let returned_value = harness.get(test_key).await.unwrap();
        assert_eq!(test_value_2, returned_value);

        let test_key = "test_key_stream";
        let test_value = Bytes::from("test_value_stream");

        let mut returned_value_stream = harness.get_stream(test_key).await.unwrap();
        let mut returned_bytes = BytesMut::new();

        while let Some(chunk) = returned_value_stream.next().await {
            let chunk = chunk.unwrap();
            returned_bytes.put(chunk);
        }

        assert_eq!(test_value, returned_bytes);

        harness.delete(test_key).await.unwrap();

        let returned_err = harness.get(test_key).await.unwrap_err();
        assert_eq!(super::ObjectStoreError::NotFound, returned_err);
    }
}
