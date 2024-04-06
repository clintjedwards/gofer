package storage

import (
	"os"
	"testing"

	"github.com/google/go-cmp/cmp"
)

func TestCRUDObjectStoreRunKeys(t *testing.T) {
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

	config := PipelineConfig{
		Namespace:  "test_namespace",
		Pipeline:   "test_pipeline",
		Version:    0,
		Registered: "0",
		Deprecated: "0",
		State:      "ACTIVE",
	}

	err = db.InsertPipelineConfig(db, &config)
	if err != nil {
		t.Fatal(err)
	}

	run := PipelineRun{
		Namespace:             "test_namespace",
		Pipeline:              "test_pipeline",
		PipelineConfigVersion: 0,
		ID:                    1,
		Started:               "0",
		Ended:                 "0",
		State:                 "STATE_STRING",
		Status:                "STATUS_STRING",
		StatusReason:          "STATUS_REASON_STRING",
		Initiator:             "EXTENSION_STRING",
		Variables:             "VARIABLES_STRING",
		StoreObjectsExpired:   false,
	}

	err = db.InsertPipelineRun(db, &run)
	if err != nil {
		t.Fatal(err)
	}

	key := ObjectStoreRunKey{
		Namespace: "test_namespace",
		Pipeline:  "test_pipeline",
		Run:       1,
		Key:       "test_key",
		Created:   "0",
	}

	err = db.InsertObjectStoreRunKey(db, &key)
	if err != nil {
		t.Fatal(err)
	}

	keys, err := db.ListObjectStoreRunKeys(db, "test_namespace", "test_pipeline", 1)
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
