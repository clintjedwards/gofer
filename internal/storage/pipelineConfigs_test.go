package storage

import (
	"errors"
	"os"
	"testing"

	"github.com/google/go-cmp/cmp"
)

func TestCRUDPipelineConfigs(t *testing.T) {
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
		Namespace:   "test_namespace",
		Pipeline:    "test_pipeline",
		Parallelism: 0,
		Name:        "test Name",
		Description: "test description",
		Version:     0,
		Registered:  0,
		Deprecated:  0,
		State:       "LIVE",
	}

	err = db.InsertPipelineConfig(db, &config)
	if err != nil {
		t.Fatal(err)
	}

	configs, err := db.ListPipelineConfigs(db, 0, 0, pipeline.Namespace, pipeline.ID)
	if err != nil {
		t.Fatal(err)
	}

	if len(configs) != 1 {
		t.Errorf("expected 1 element in list found %d", len(configs))
	}

	if diff := cmp.Diff(config, configs[0]); diff != "" {
		t.Errorf("unexpected map values (-want +got):\n%s", diff)
	}

	liveConfig, err := db.GetLatestLivePipelineConfig(db, pipeline.Namespace, pipeline.ID)
	if err != nil {
		t.Fatal(err)
	}

	if diff := cmp.Diff(config, liveConfig); diff != "" {
		t.Errorf("unexpected map values (-want +got):\n%s", diff)
	}

	fetchedConfig, err := db.GetPipelineConfig(db, namespace.ID, pipeline.ID, config.Version)
	if err != nil {
		t.Fatal(err)
	}

	if diff := cmp.Diff(config, fetchedConfig); diff != "" {
		t.Errorf("unexpected map values (-want +got):\n%s", diff)
	}

	fetchedConfig, err = db.GetLatestPipelineConfig(db, namespace.ID, pipeline.ID)
	if err != nil {
		t.Fatal(err)
	}

	if diff := cmp.Diff(config, fetchedConfig); diff != "" {
		t.Errorf("unexpected map values (-want +got):\n%s", diff)
	}

	fetchedConfig.Deprecated = 1
	fetchedConfig.State = "DISABLED"

	err = db.UpdatePipelineConfig(db, namespace.ID, pipeline.ID, config.Version,
		UpdatablePipelineConfigFields{
			Deprecated: &fetchedConfig.Deprecated,
			State:      &fetchedConfig.State,
		})
	if err != nil {
		t.Fatal(err)
	}

	updatedConfig, err := db.GetPipelineConfig(db, namespace.ID, pipeline.ID, config.Version)
	if err != nil {
		t.Fatal(err)
	}

	if diff := cmp.Diff(fetchedConfig, updatedConfig); diff != "" {
		t.Errorf("unexpected map values (-want +got):\n%s", diff)
	}

	err = db.DeletePipelineConfig(db, namespace.ID, pipeline.ID, config.Version)
	if err != nil {
		t.Fatal(err)
	}

	_, err = db.GetPipelineConfig(db, namespace.ID, pipeline.ID, config.Version)
	if !errors.Is(err, ErrEntityNotFound) {
		t.Fatal("expected error Not Found; found alternate error")
	}
}
