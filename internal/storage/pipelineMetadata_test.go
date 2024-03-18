package storage

import (
	"errors"
	"os"
	"testing"

	"github.com/google/go-cmp/cmp"
)

func TestCRUDPipelines(t *testing.T) {
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

	pipelines, err := db.ListPipelineMetadata(db, 0, 0, pipeline.Namespace)
	if err != nil {
		t.Fatal(err)
	}

	if len(pipelines) != 1 {
		t.Errorf("expected 1 element in list found %d", len(pipelines))
	}

	if diff := cmp.Diff(pipeline, pipelines[0]); diff != "" {
		t.Errorf("unexpected map values (-want +got):\n%s", diff)
	}

	pipelineCount, err := db.GetPipelineCount(db)
	if err != nil {
		t.Fatal(err)
	}

	if pipelineCount != int64(len(pipelines)) {
		t.Errorf("unexpected pipeline count")
	}

	fetchedPipeline, err := db.GetPipelineMetadata(db, namespace.ID, pipeline.ID)
	if err != nil {
		t.Fatal(err)
	}

	if diff := cmp.Diff(pipeline, fetchedPipeline); diff != "" {
		t.Errorf("unexpected map values (-want +got):\n%s", diff)
	}

	fetchedPipeline.Modified = 1
	fetchedPipeline.State = "DISABLED"

	err = db.UpdatePipelineMetadata(db, namespace.ID, pipeline.ID,
		UpdatablePipelineMetadataFields{
			Modified: &fetchedPipeline.Modified,
			State:    &fetchedPipeline.State,
		})
	if err != nil {
		t.Fatal(err)
	}

	updatedPipeline, err := db.GetPipelineMetadata(db, namespace.ID, pipeline.ID)
	if err != nil {
		t.Fatal(err)
	}

	if diff := cmp.Diff(fetchedPipeline, updatedPipeline); diff != "" {
		t.Errorf("unexpected map values (-want +got):\n%s", diff)
	}

	err = db.DeletePipelineMetadata(db, namespace.ID, pipeline.ID)
	if err != nil {
		t.Fatal(err)
	}

	_, err = db.GetPipelineMetadata(db, namespace.ID, pipeline.ID)
	if !errors.Is(err, ErrEntityNotFound) {
		t.Fatal("expected error Not Found; found alternate error")
	}
}
