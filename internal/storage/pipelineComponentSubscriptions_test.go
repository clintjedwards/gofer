package storage

import (
	"errors"
	"os"
	"testing"

	"github.com/google/go-cmp/cmp"
)

func TestCRUDPipelineExtensionSubscriptions(t *testing.T) {
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
		Created:     0,
		Modified:    0,
	}

	err = db.InsertNamespace(db, &namespace)
	if err != nil {
		t.Fatal(err)
	}

	pipeline := PipelineMetadata{
		Namespace: "test_namespace",
		ID:        "test_pipeline",
		Created:   0,
		Modified:  0,
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
		Registered: 0,
		Deprecated: 0,
		State:      "ACTIVE",
	}

	err = db.InsertPipelineConfig(db, &config)
	if err != nil {
		t.Fatal(err)
	}

	sub := PipelineExtensionSubscription{
		Namespace:    "test_namespace",
		Pipeline:     "test_pipeline",
		Name:         "test name",
		Label:        "test_label",
		Settings:     "settings string",
		Status:       "ACTIVE",
		StatusReason: "reason string",
	}

	err = db.InsertPipelineExtensionSubscription(db, &sub)
	if err != nil {
		t.Fatal(err)
	}

	subs, err := db.ListPipelineExtensionSubscriptions(db, pipeline.Namespace, pipeline.ID)
	if err != nil {
		t.Fatal(err)
	}

	if len(subs) != 1 {
		t.Errorf("expected 1 element in list found %d", len(subs))
	}

	if diff := cmp.Diff(sub, subs[0]); diff != "" {
		t.Errorf("unexpected map values (-want +got):\n%s", diff)
	}

	fetchedTask, err := db.GetPipelineExtensionSubscription(db, namespace.ID, pipeline.ID, sub.Name, sub.Label)
	if err != nil {
		t.Fatal(err)
	}

	if diff := cmp.Diff(sub, fetchedTask); diff != "" {
		t.Errorf("unexpected map values (-want +got):\n%s", diff)
	}

	err = db.DeletePipelineExtensionSubscription(db, namespace.ID, pipeline.ID, sub.Name, sub.Label)
	if err != nil {
		t.Fatal(err)
	}

	_, err = db.GetPipelineExtensionSubscription(db, namespace.ID, pipeline.ID, sub.Name, sub.Label)
	if !errors.Is(err, ErrEntityNotFound) {
		t.Fatal("expected error Not Found; found alternate error")
	}
}
