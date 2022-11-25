package storage

import (
	"os"
	"testing"

	"github.com/google/go-cmp/cmp"
)

func TestCRUDPipelineCommonTaskSettings(t *testing.T) {
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

	settings := PipelineCommonTaskSettings{
		Namespace:             "test_namespace",
		Pipeline:              "test_pipeline",
		PipelineConfigVersion: 0,
		Description:           "test_description",
		DependsOn:             "test_depends_on",
		Name:                  "test_common_task_settings_name",
		Label:                 "test_common_task_settings_label",
		Settings:              "test_settings",
		InjectAPIToken:        true,
	}

	err = db.InsertPipelineCommonTaskSettings(db, &settings)
	if err != nil {
		t.Fatal(err)
	}

	settingsList, err := db.ListPipelineCommonTaskSettings(db, pipeline.Namespace, pipeline.ID, config.Version)
	if err != nil {
		t.Fatal(err)
	}

	if len(settingsList) != 1 {
		t.Errorf("expected 1 element in list found %d", len(settingsList))
	}

	if diff := cmp.Diff(settings, settingsList[0]); diff != "" {
		t.Errorf("unexpected map values (-want +got):\n%s", diff)
	}

	fetchedSettings, err := db.GetPipelineCommonTaskSettings(db, namespace.ID, pipeline.ID, config.Version,
		settings.Label)
	if err != nil {
		t.Fatal(err)
	}

	if diff := cmp.Diff(settings, fetchedSettings); diff != "" {
		t.Errorf("unexpected map values (-want +got):\n%s", diff)
	}
}
