package storage

import (
	"errors"
	"os"
	"testing"

	"github.com/google/go-cmp/cmp"
)

func TestCRUDSecretStoreGlobalKeys(t *testing.T) {
	path := tempFile()
	db, err := New(path, 200)
	if err != nil {
		t.Fatal(err)
	}
	defer os.Remove(path)

	key := SecretStoreGlobalKey{
		Key:        "test_key",
		Namespaces: "namespace,test",
		Created:    0,
	}

	err = db.InsertSecretStoreGlobalKey(db, &key, false)
	if err != nil {
		t.Fatal(err)
	}

	keys, err := db.ListSecretStoreGlobalKeys(db)
	if err != nil {
		t.Fatal(err)
	}

	if len(keys) != 1 {
		t.Errorf("expected 1 element in list found %d", len(keys))
	}

	if diff := cmp.Diff(key, keys[0]); diff != "" {
		t.Errorf("unexpected map values (-want +got):\n%s", diff)
	}

	fetchedKey, err := db.GetSecretStoreGlobalKey(db, key.Key)
	if err != nil {
		t.Fatal(err)
	}

	if diff := cmp.Diff(key, fetchedKey); diff != "" {
		t.Errorf("unexpected map values (-want +got):\n%s", diff)
	}

	key.Namespaces = "namespace,test2"

	err = db.UpdateSecretStoreGlobalKey(db, key.Key, UpdatableSecretStoreGlobalKeyFields{
		Namespaces: &key.Namespaces,
	})
	if err != nil {
		t.Fatal(err)
	}

	fetchedKey, err = db.GetSecretStoreGlobalKey(db, key.Key)
	if err != nil {
		t.Fatal(err)
	}

	if diff := cmp.Diff(key, fetchedKey); diff != "" {
		t.Errorf("unexpected map values (-want +got):\n%s", diff)
	}

	err = db.DeleteSecretStoreGlobalKey(db, key.Key)
	if err != nil {
		t.Fatal(err)
	}

	_, err = db.GetSecretStoreGlobalKey(db, key.Key)
	if !errors.Is(err, ErrEntityNotFound) {
		t.Fatal("expected error Not Found; found alternate error")
	}
}
