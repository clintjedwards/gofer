use crate::secret_store::Store;
use defer_lite::defer;
use rand::prelude::*;

#[tokio::test]
/// Basic CRUD can be accomplished for the embedded secret store.
async fn crud_secret_store_embedded() {
    let mut rng = rand::thread_rng();
    let append_num: u8 = rng.gen();
    let path = format!("/tmp/gofer_tests_embedded_secret_store{}", append_num);
    defer! {std::fs::remove_dir_all(&path).unwrap();};

    let store = super::embedded::Engine::new(&path, "changemechangemechangemechangeme")
        .await
        .unwrap();

    let test_key = "test_key";
    let test_value = "test_value";

    store.put_secret(test_key, test_value, false).await.unwrap();

    let returned_value = store.get_secret(test_key).await.unwrap();
    assert_eq!(test_value, String::from_utf8_lossy(&returned_value));

    store.delete_secret(test_key).await.unwrap();

    let returned_err = store.get_secret(test_key).await.unwrap_err();
    assert_eq!(super::SecretStoreError::NotFound, returned_err);
}
