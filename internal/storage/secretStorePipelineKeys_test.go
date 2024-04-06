package storage

import (
	"os"
	"testing"

	"github.com/google/go-cmp/cmp"
)

func TestCRUDSecretStorePipelineKeys(t *testing.T) {
	path := tempFile()
	db, err := New(path, 200)
	if err != nil {
		t.Fatal(err)
	}
	defer os.Remove(path)

	namespace := Namespace{
		ID:          "test_namespace",
		Name:        "Test Namespace",
		Description: "This is a test namespace",
		Created:     "0",
		Modified:    "0",
	}

	err = db.InsertNamespace(db, &namespace)
	if err != nil {
		t.Fatal(err)
	}

	pipeline := PipelineMetadata{
		Namespace: "test_namespace",
		ID:        "test_pipeline",
		Created:   "0",
		Modified:  "0",
		State:     "ACTIVE",
	}

	err = db.InsertPipelineMetadata(db, &pipeline)
	if err != nil {
		t.Fatal(err)
	}

	key := SecretStorePipelineKey{
		Namespace: "test_namespace",
		Pipeline:  "test_pipeline",
		Key:       "test_key",
		Created:   "0",
	}

	err = db.InsertSecretStorePipelineKey(db, &key, false)
	if err != nil {
		t.Fatal(err)
	}

	keys, err := db.ListSecretStorePipelineKeys(db, "test_namespace", "test_pipeline")
	if err != nil {
		t.Fatal(err)
	}

	if len(keys) != 1 {
		t.Errorf("expected 1 element in list found %d", len(keys))
	}

	if diff := cmp.Diff(key, keys[0]); diff != "" {
		t.Errorf("unexpected map values (-want +got):\n%s", diff)
	}
}
