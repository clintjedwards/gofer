package storage

import (
	"os"
	"testing"

	"github.com/google/go-cmp/cmp"
)

func TestCRUDPipelineTasks(t *testing.T) {
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

	task := PipelineTask{
		Namespace:             "test_namespace",
		Pipeline:              "test_pipeline",
		PipelineConfigVersion: 0,
		ID:                    "test_task",
		Description:           "test description",
		Image:                 "test",
		RegistryAuth:          "auth string",
		DependsOn:             "depends on list",
		Variables:             "variable list",
		Entrypoint:            "entrypoint list",
		Command:               "command list",
		InjectAPIToken:        true,
	}

	err = db.InsertPipelineTask(db, &task)
	if err != nil {
		t.Fatal(err)
	}

	taskList, err := db.ListPipelineTasks(db, pipeline.Namespace, pipeline.ID, config.Version)
	if err != nil {
		t.Fatal(err)
	}

	if len(taskList) != 1 {
		t.Errorf("expected 1 element in list found %d", len(taskList))
	}

	if diff := cmp.Diff(task, taskList[0]); diff != "" {
		t.Errorf("unexpected map values (-want +got):\n%s", diff)
	}

	fetchedTask, err := db.GetPipelineTask(db, namespace.ID, pipeline.ID, config.Version,
		task.ID)
	if err != nil {
		t.Fatal(err)
	}

	if diff := cmp.Diff(task, fetchedTask); diff != "" {
		t.Errorf("unexpected map values (-want +got):\n%s", diff)
	}
}
