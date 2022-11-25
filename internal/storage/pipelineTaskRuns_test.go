package storage

import (
	"errors"
	"os"
	"testing"

	"github.com/google/go-cmp/cmp"
)

func TestCRUDPipelineTaskRuns(t *testing.T) {
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

	run := PipelineRun{
		Namespace:             "test_namespace",
		Pipeline:              "test_pipeline",
		PipelineConfigVersion: 0,
		ID:                    1,
		Started:               0,
		Ended:                 0,
		State:                 "STATE_STRING",
		Status:                "STATUS_STRING",
		StatusReason:          "STATUS_REASON_STRING",
		Extension:             "EXTENSION_STRING",
		Variables:             "VARIABLES_STRING",
		StoreObjectsExpired:   false,
	}

	err = db.InsertPipelineRun(db, &run)
	if err != nil {
		t.Fatal(err)
	}

	taskRun := PipelineTaskRun{
		Namespace:    "test_namespace",
		Pipeline:     "test_pipeline",
		Run:          1,
		ID:           "test_task_run",
		Task:         "TASK_STRING",
		Created:      0,
		Started:      0,
		Ended:        0,
		ExitCode:     0,
		LogsExpired:  true,
		LogsRemoved:  true,
		State:        "STATE_STRING",
		Status:       "STATUS_STRING",
		StatusReason: "STATUS_REASON_STRING",
		Variables:    "VARIABLES_STRING",
	}

	err = db.InsertPipelineTaskRun(db, &taskRun)
	if err != nil {
		t.Fatal(err)
	}

	taskRuns, err := db.ListPipelineTaskRuns(db, 0, 0, pipeline.Namespace, pipeline.ID, run.ID)
	if err != nil {
		t.Fatal(err)
	}

	if len(taskRuns) != 1 {
		t.Errorf("expected 1 element in list found %d", len(taskRuns))
	}

	if diff := cmp.Diff(taskRun, taskRuns[0]); diff != "" {
		t.Errorf("unexpected map values (-want +got):\n%s", diff)
	}

	fetchedRun, err := db.GetPipelineTaskRun(db, namespace.ID, pipeline.ID, run.ID, taskRun.ID)
	if err != nil {
		t.Fatal(err)
	}

	if diff := cmp.Diff(taskRun, fetchedRun); diff != "" {
		t.Errorf("unexpected map values (-want +got):\n%s", diff)
	}

	fetchedRun.Ended = 1
	fetchedRun.State = "DISABLED"
	fetchedRun.Status = "UPDATED_STATUS"
	fetchedRun.StatusReason = "UPDATED_STATUS_REASON"
	fetchedRun.Variables = "UPDATED_VARIABLES"
	fetchedRun.LogsExpired = false

	err = db.UpdatePipelineTaskRun(db, namespace.ID, pipeline.ID, run.ID, taskRun.ID,
		UpdatablePipelineTaskRunFields{
			Ended:        &fetchedRun.Ended,
			State:        &fetchedRun.State,
			Status:       &fetchedRun.Status,
			StatusReason: &fetchedRun.StatusReason,
			Variables:    &fetchedRun.Variables,
			LogsExpired:  &fetchedRun.LogsExpired,
		})
	if err != nil {
		t.Fatal(err)
	}

	updatedRun, err := db.GetPipelineTaskRun(db, namespace.ID, pipeline.ID, run.ID, taskRun.ID)
	if err != nil {
		t.Fatal(err)
	}

	if diff := cmp.Diff(fetchedRun, updatedRun); diff != "" {
		t.Errorf("unexpected map values (-want +got):\n%s", diff)
	}

	err = db.DeletePipelineTaskRun(db, namespace.ID, pipeline.ID, run.ID, taskRun.ID)
	if err != nil {
		t.Fatal(err)
	}

	_, err = db.GetPipelineTaskRun(db, namespace.ID, pipeline.ID, run.ID, taskRun.ID)
	if !errors.Is(err, ErrEntityNotFound) {
		t.Fatal("expected error Not Found; found alternate error")
	}
}
