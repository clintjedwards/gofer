use rand::prelude::*;
use defer_lite::defer;
use crate::object_store::Store;

#[tokio::test]
/// Basic CRUD can be accomplished for the embedded object store.
async fn crud_object_store_embedded() {

    let mut rng = rand::thread_rng();
    let append_num: u8 = rng.gen();
    let path = format!("/tmp/gofer_tests_embedded_object_store{}", append_num);
    defer!{std::fs::remove_dir_all(&path).unwrap();};

    let store = super::embedded::Engine::new(&path).await.unwrap();

    let test_key = "test_key";
    let test_value = "test_value".as_bytes();

    store.put_object(test_key, test_value.to_vec(), false).await.unwrap();

    let returned_value = store.get_object(test_key).await.unwrap();
    assert_eq!(test_value, returned_value);

    store.delete_object(test_key).await.unwrap();

    let returned_err = store.get_object(test_key).await.unwrap_err();
    assert_eq!(super::ObjectStoreError::NotFound, returned_err);
}
