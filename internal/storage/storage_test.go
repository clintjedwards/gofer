package storage

import (
	"errors"
	"os"
	"testing"

	"github.com/clintjedwards/gofer/models"
	sdk "github.com/clintjedwards/gofer/sdk/go"
	"github.com/google/go-cmp/cmp"
)

func tempFile() string {
	f, err := os.CreateTemp("", "gofer-test-")
	if err != nil {
		panic(err)
	}
	if err := f.Close(); err != nil {
		panic(err)
	}
	if err := os.Remove(f.Name()); err != nil {
		panic(err)
	}
	return f.Name()
}

func TestCRUDNamespaces(t *testing.T) {
	path := tempFile()
	db, err := New(path, 200)
	if err != nil {
		t.Fatal(err)
	}
	defer os.Remove(path)

	namespace := models.NewNamespace("test_namespace", "Test Namespace", "Testing namespace")

	err = db.InsertNamespace(namespace)
	if err != nil {
		t.Fatal(err)
	}

	namespaces, err := db.ListNamespaces(0, 0)
	if err != nil {
		t.Fatal(err)
	}

	if diff := cmp.Diff(*namespace, namespaces[0]); diff != "" {
		t.Errorf("unexpected map values (-want +got):\n%s", diff)
	}

	fetchedNamespace, err := db.GetNamespace(namespace.ID)
	if err != nil {
		t.Fatal(err)
	}

	if diff := cmp.Diff(*namespace, fetchedNamespace); diff != "" {
		t.Errorf("unexpected map values (-want +got):\n%s", diff)
	}

	err = db.UpdateNamespace(namespace.ID, UpdatableNamespaceFields{
		Description: ptr("updated namespace"),
	})
	if err != nil {
		t.Fatal(err)
	}

	namespace.Description = "updated namespace"

	fetchedNamespace, err = db.GetNamespace(namespace.ID)
	if err != nil {
		t.Fatal(err)
	}

	if diff := cmp.Diff(*namespace, fetchedNamespace); diff != "" {
		t.Errorf("unexpected map values (-want +got):\n%s", diff)
	}

	err = db.DeleteNamespace(namespace.ID)
	if err != nil {
		t.Fatal(err)
	}

	_, err = db.GetNamespace(namespace.ID)
	if !errors.Is(err, ErrEntityNotFound) {
		t.Fatal("expected error Not Found; found alternate error")
	}
}

func TestCRUDPipelines(t *testing.T) {
	path := tempFile()
	db, err := New(path, 200)
	if err != nil {
		t.Fatal(err)
	}
	defer os.Remove(path)

	namespace := models.NewNamespace("test_namespace", "Test Namespace", "Testing namespace")
	err = db.InsertNamespace(namespace)
	if err != nil {
		t.Fatal(err)
	}

	pipelineConfig := sdk.NewPipeline("test_pipeline", "Test Pipeline").WithTasks([]sdk.Task{
		*sdk.NewTask("test_task", "task:latest").WithDependsOnOne("test_task_depends", sdk.RequiredParentStatusAny),
	}).WithTriggers([]sdk.PipelineTriggerConfig{
		{
			Name:  "test_trigger",
			Label: "test_trigger_label",
			Settings: map[string]string{
				"test_setting_key": "test_setting_value",
			},
		},
	})
	pipeline := models.NewPipeline(namespace.ID, pipelineConfig)

	err = db.InsertPipeline(pipeline)
	if err != nil {
		t.Fatal(err)
	}

	pipelines, err := db.ListPipelines(0, 0, namespace.ID)
	if err != nil {
		t.Fatal(err)
	}

	if diff := cmp.Diff(*pipeline, pipelines[0]); diff != "" {
		t.Errorf("unexpected map values (-want +got):\n%s", diff)
	}

	fetchedPipeline, err := db.GetPipeline(nil, namespace.ID, pipeline.ID)
	if err != nil {
		t.Fatal(err)
	}

	if diff := cmp.Diff(*pipeline, fetchedPipeline); diff != "" {
		t.Errorf("unexpected map values (-want +got):\n%s", diff)
	}

	fetchedPipeline.Description = "updated pipeline"
	fetchedPipeline.Tasks = map[string]models.Task{
		"updated_task": {
			ID:    "updated_task",
			Image: "updated:latest",
			DependsOn: map[string]models.RequiredParentStatus{
				"test_task_depends": models.RequiredParentStatusAny,
			},
		},
	}

	err = db.UpdatePipeline(namespace.ID, pipeline.ID, UpdatablePipelineFields{
		Description: ptr("updated pipeline"),
		Tasks:       &fetchedPipeline.Tasks,
	})
	if err != nil {
		t.Fatal(err)
	}

	updatedPipeline, err := db.GetPipeline(nil, namespace.ID, pipeline.ID)
	if err != nil {
		t.Fatal(err)
	}

	if diff := cmp.Diff(fetchedPipeline, updatedPipeline); diff != "" {
		t.Errorf("unexpected map values (-want +got):\n%s", diff)
	}

	err = db.DeletePipeline(namespace.ID, pipeline.ID)
	if err != nil {
		t.Fatal(err)
	}

	_, err = db.GetPipeline(nil, namespace.ID, pipeline.ID)
	if !errors.Is(err, ErrEntityNotFound) {
		t.Fatal("expected error Not Found; found alternate error")
	}
}

func TestCRUDRuns(t *testing.T) {
	path := tempFile()
	db, err := New(path, 200)
	if err != nil {
		t.Fatal(err)
	}
	defer os.Remove(path)

	namespace := models.NewNamespace("test_namespace", "Test Namespace", "Testing namespace")

	err = db.InsertNamespace(namespace)
	if err != nil {
		t.Fatal(err)
	}

	pipelineConfig := sdk.NewPipeline("test_pipeline", "Test Pipeline").WithTasks([]sdk.Task{
		*sdk.NewTask("test_task", "task:latest").WithDependsOnOne("test_task_depends", sdk.RequiredParentStatusAny),
	}).WithTriggers([]sdk.PipelineTriggerConfig{
		{
			Name:  "test_trigger",
			Label: "test_trigger_label",
			Settings: map[string]string{
				"test_setting_key": "test_setting_value",
			},
		},
	})
	pipeline := models.NewPipeline(namespace.ID, pipelineConfig)

	err = db.InsertPipeline(pipeline)
	if err != nil {
		t.Fatal(err)
	}

	run := models.NewRun(namespace.ID, pipeline.ID, models.TriggerInfo{
		Name:  "test_trigger_name",
		Label: "test_trigger_label",
	}, []models.Variable{})
	run.TaskRuns = []string{
		"test_task_run",
	}

	runID, err := db.InsertRun(run)
	if err != nil {
		t.Fatal(err)
	}
	run.ID = runID

	runs, err := db.ListRuns(nil, 0, 0, namespace.ID, pipeline.ID)
	if err != nil {
		t.Fatal(err)
	}

	if diff := cmp.Diff(*run, runs[0]); diff != "" {
		t.Errorf("unexpected map values (-want +got):\n%s", diff)
	}

	fetchedRun, err := db.GetRun(namespace.ID, pipeline.ID, run.ID)
	if err != nil {
		t.Fatal(err)
	}

	if diff := cmp.Diff(*run, fetchedRun); diff != "" {
		t.Errorf("unexpected map values (-want +got):\n%s", diff)
	}

	err = db.UpdateRun(namespace.ID, pipeline.ID, fetchedRun.ID, UpdatableRunFields{
		State:  ptr(models.RunStateComplete),
		Status: ptr(models.RunStatusSuccessful),
	})
	if err != nil {
		t.Fatal(err)
	}

	run.State = models.RunStateComplete
	run.Status = models.RunStatusSuccessful

	fetchedRun, err = db.GetRun(namespace.ID, pipeline.ID, run.ID)
	if err != nil {
		t.Fatal(err)
	}

	if diff := cmp.Diff(*run, fetchedRun); diff != "" {
		t.Errorf("unexpected map values (-want +got):\n%s", diff)
	}

	err = db.DeleteRun(run.Namespace, run.Pipeline, run.ID)
	if err != nil {
		t.Fatal(err)
	}

	_, err = db.GetRun(run.Namespace, run.Pipeline, run.ID)
	if !errors.Is(err, ErrEntityNotFound) {
		t.Fatal("expected error Not Found; found alternate error")
	}
}

func TestCRUDTaskRuns(t *testing.T) {
	path := tempFile()
	db, err := New(path, 200)
	if err != nil {
		t.Fatal(err)
	}
	defer os.Remove(path)

	namespace := models.NewNamespace("test_namespace", "Test Namespace", "Testing namespace")

	err = db.InsertNamespace(namespace)
	if err != nil {
		t.Fatal(err)
	}

	pipelineConfig := sdk.NewPipeline("test_pipeline", "Test Pipeline").WithTasks([]sdk.Task{
		*sdk.NewTask("test_task", "task:latest").WithDependsOnOne("test_task_depends", sdk.RequiredParentStatusAny),
	}).WithTriggers([]sdk.PipelineTriggerConfig{
		{
			Name:  "test_trigger",
			Label: "test_trigger_label",
			Settings: map[string]string{
				"test_setting_key": "test_setting_value",
			},
		},
	})
	pipeline := models.NewPipeline(namespace.ID, pipelineConfig)

	err = db.InsertPipeline(pipeline)
	if err != nil {
		t.Fatal(err)
	}

	run := models.NewRun(namespace.ID, pipeline.ID, models.TriggerInfo{
		Name:  "test_trigger_name",
		Label: "test_trigger_label",
	}, []models.Variable{})
	run.TaskRuns = []string{
		"test_task_run",
	}

	runID, err := db.InsertRun(run)
	if err != nil {
		t.Fatal(err)
	}
	run.ID = runID

	taskRun := models.NewTaskRun(namespace.ID, pipeline.ID, run.ID, models.Task{
		ID: "test_task",
	})

	err = db.InsertTaskRun(taskRun)
	if err != nil {
		t.Fatal(err)
	}

	taskRuns, err := db.ListTaskRuns(0, 0, namespace.ID, pipeline.ID, run.ID)
	if err != nil {
		t.Fatal(err)
	}

	if diff := cmp.Diff(*taskRun, taskRuns[0]); diff != "" {
		t.Errorf("unexpected map values (-want +got):\n%s", diff)
	}

	fetchedTaskRun, err := db.GetTaskRun(namespace.ID, pipeline.ID, run.ID, taskRun.ID)
	if err != nil {
		t.Fatal(err)
	}

	if diff := cmp.Diff(*taskRun, fetchedTaskRun); diff != "" {
		t.Errorf("unexpected map values (-want +got):\n%s", diff)
	}

	err = db.UpdateTaskRun(&fetchedTaskRun, UpdatableTaskRunFields{
		State: ptr(models.TaskRunStateComplete),
	})
	if err != nil {
		t.Fatal(err)
	}

	taskRun.State = models.TaskRunStateComplete

	fetchedTaskRun, err = db.GetTaskRun(namespace.ID, pipeline.ID, run.ID, taskRun.ID)
	if err != nil {
		t.Fatal(err)
	}

	if diff := cmp.Diff(*taskRun, fetchedTaskRun); diff != "" {
		t.Errorf("unexpected map values (-want +got):\n%s", diff)
	}

	err = db.DeleteTaskRun(run.Namespace, run.Pipeline, run.ID, taskRun.ID)
	if err != nil {
		t.Fatal(err)
	}

	_, err = db.GetTaskRun(run.Namespace, run.Pipeline, run.ID, taskRun.ID)
	if !errors.Is(err, ErrEntityNotFound) {
		t.Fatal("expected error Not Found; found alternate error")
	}
}

func TestCRUDEvents(t *testing.T) {
	path := tempFile()
	db, err := New(path, 200)
	if err != nil {
		t.Fatal(err)
	}
	defer os.Remove(path)

	event := models.NewEvent(&models.EventCreatedNamespace{
		NamespaceID: "test_namespace",
	})

	eventID, err := db.InsertEvent(event)
	if err != nil {
		t.Fatal(err)
	}

	event.ID = eventID

	events, err := db.ListEvents(0, 0, false)
	if err != nil {
		t.Fatal(err)
	}

	if diff := cmp.Diff(*event, events[0]); diff != "" {
		t.Errorf("unexpected map values (-want +got):\n%s", diff)
	}

	fetchedEvent, err := db.GetEvent(event.ID)
	if err != nil {
		t.Fatal(err)
	}

	if diff := cmp.Diff(*event, fetchedEvent); diff != "" {
		t.Errorf("unexpected map values (-want +got):\n%s", diff)
	}

	err = db.DeleteEvent(event.ID)
	if err != nil {
		t.Fatal(err)
	}

	_, err = db.GetEvent(event.ID)
	if !errors.Is(err, ErrEntityNotFound) {
		t.Fatal("expected error Not Found; found alternate error")
	}
}

func TestCRUDTriggerRegistrations(t *testing.T) {
	path := tempFile()
	db, err := New(path, 200)
	if err != nil {
		t.Fatal(err)
	}
	defer os.Remove(path)

	triggerReg := models.TriggerRegistration{
		Name: "test_trigger_registration",
		Variables: []models.Variable{
			{
				Key:    "key",
				Value:  "value",
				Source: models.VariableSourceSystem,
			},
		},
	}

	err = db.InsertTriggerRegistration(&triggerReg)
	if err != nil {
		t.Fatal(err)
	}

	regs, err := db.ListTriggerRegistrations(0, 0)
	if err != nil {
		t.Fatal(err)
	}

	if diff := cmp.Diff(triggerReg, regs[0]); diff != "" {
		t.Errorf("unexpected map values (-want +got):\n%s", diff)
	}

	fetchedReg, err := db.GetTriggerRegistration(triggerReg.Name)
	if err != nil {
		t.Fatal(err)
	}

	if diff := cmp.Diff(triggerReg, fetchedReg); diff != "" {
		t.Errorf("unexpected map values (-want +got):\n%s", diff)
	}

	err = db.UpdateTriggerRegistration(triggerReg.Name, UpdatableTriggerRegistrationFields{
		Image: ptr("image:latest"),
	})
	if err != nil {
		t.Fatal(err)
	}

	triggerReg.Image = "image:latest"

	fetchedReg, err = db.GetTriggerRegistration(triggerReg.Name)
	if err != nil {
		t.Fatal(err)
	}

	if diff := cmp.Diff(triggerReg, fetchedReg); diff != "" {
		t.Errorf("unexpected map values (-want +got):\n%s", diff)
	}

	err = db.DeleteTriggerRegistration(triggerReg.Name)
	if err != nil {
		t.Fatal(err)
	}

	_, err = db.GetTriggerRegistration(triggerReg.Name)
	if !errors.Is(err, ErrEntityNotFound) {
		t.Fatalf("expected error Not Found; found error %v", err)
	}
}

func TestCRUDCommonTaskRegistrations(t *testing.T) {
	path := tempFile()
	db, err := New(path, 200)
	if err != nil {
		t.Fatal(err)
	}
	defer os.Remove(path)

	commonTaskReg := models.CommonTaskRegistration{
		Name: "test_commontask_registration",
		Variables: []models.Variable{
			{
				Key:    "key",
				Value:  "value",
				Source: models.VariableSourceSystem,
			},
		},
	}

	err = db.InsertCommonTaskRegistration(&commonTaskReg)
	if err != nil {
		t.Fatal(err)
	}

	regs, err := db.ListCommonTaskRegistrations(0, 0)
	if err != nil {
		t.Fatal(err)
	}

	if diff := cmp.Diff(commonTaskReg, regs[0]); diff != "" {
		t.Errorf("unexpected map values (-want +got):\n%s", diff)
	}

	fetchedReg, err := db.GetCommonTaskRegistration(commonTaskReg.Name)
	if err != nil {
		t.Fatal(err)
	}

	if diff := cmp.Diff(commonTaskReg, fetchedReg); diff != "" {
		t.Errorf("unexpected map values (-want +got):\n%s", diff)
	}

	err = db.UpdateCommonTaskRegistration(commonTaskReg.Name, UpdatableCommonTaskRegistrationFields{
		Image: ptr("image:latest"),
	})
	if err != nil {
		t.Fatal(err)
	}

	commonTaskReg.Image = "image:latest"

	fetchedReg, err = db.GetCommonTaskRegistration(commonTaskReg.Name)
	if err != nil {
		t.Fatal(err)
	}

	if diff := cmp.Diff(commonTaskReg, fetchedReg); diff != "" {
		t.Errorf("unexpected map values (-want +got):\n%s", diff)
	}

	err = db.DeleteCommonTaskRegistration(commonTaskReg.Name)
	if err != nil {
		t.Fatal(err)
	}

	_, err = db.GetCommonTaskRegistration(commonTaskReg.Name)
	if !errors.Is(err, ErrEntityNotFound) {
		t.Fatalf("expected error Not Found; found error %v", err)
	}
}

func TestCRUDTokens(t *testing.T) {
	path := tempFile()
	db, err := New(path, 200)
	if err != nil {
		t.Fatal(err)
	}
	defer os.Remove(path)

	token := models.NewToken("test_hash", models.TokenKindManagement, []string{"test_namespace"}, map[string]string{"key": "value"})

	err = db.InsertToken(token)
	if err != nil {
		t.Fatal(err)
	}

	tokens, err := db.ListTokens(0, 0)
	if err != nil {
		t.Fatal(err)
	}

	if diff := cmp.Diff(*token, tokens[0]); diff != "" {
		t.Errorf("unexpected map values (-want +got):\n%s", diff)
	}

	fetchedToken, err := db.GetToken(token.Hash)
	if err != nil {
		t.Fatal(err)
	}

	if diff := cmp.Diff(*token, fetchedToken); diff != "" {
		t.Errorf("unexpected map values (-want +got):\n%s", diff)
	}

	err = db.DeleteToken(token.Hash)
	if err != nil {
		t.Fatal(err)
	}

	_, err = db.GetToken(token.Hash)
	if !errors.Is(err, ErrEntityNotFound) {
		t.Fatal("expected error Not Found; found alternate error")
	}
}

func TestCRUDObjectStorePipelineKeys(t *testing.T) {
	path := tempFile()
	db, err := New(path, 200)
	if err != nil {
		t.Fatal(err)
	}
	defer os.Remove(path)

	namespace := models.NewNamespace("test_namespace", "Test Namespace", "Testing namespace")

	err = db.InsertNamespace(namespace)
	if err != nil {
		t.Fatal(err)
	}

	pipelineConfig := sdk.NewPipeline("test_pipeline", "Test Pipeline").WithTasks([]sdk.Task{
		*sdk.NewTask("test_task", "task:latest").WithDependsOnOne("test_task_depends", sdk.RequiredParentStatusAny),
	}).WithTriggers([]sdk.PipelineTriggerConfig{
		{
			Name:  "test_trigger",
			Label: "test_trigger_label",
			Settings: map[string]string{
				"test_setting_key": "test_setting_value",
			},
		},
	})
	pipeline := models.NewPipeline(namespace.ID, pipelineConfig)

	err = db.InsertPipeline(pipeline)
	if err != nil {
		t.Fatal(err)
	}

	key := models.NewObjectStoreKey("test_key")

	err = db.InsertObjectStorePipelineKey("test_namespace", "test_pipeline", key)
	if err != nil {
		t.Fatal(err)
	}

	keys, err := db.ListObjectStorePipelineKeys("test_namespace", "test_pipeline")
	if err != nil {
		t.Fatal(err)
	}

	if diff := cmp.Diff(*key, keys[0]); diff != "" {
		t.Errorf("unexpected map values (-want +got):\n%s", diff)
	}

	err = db.DeleteObjectStorePipelineKey("test_namespace", "test_pipeline", key.Key)
	if err != nil {
		t.Fatal(err)
	}

	keys, err = db.ListObjectStorePipelineKeys("test_namespace", "test_pipeline")
	if err != nil {
		t.Fatal(err)
	}

	if len(keys) != 0 {
		t.Fatalf("expected 0 keys but found keys: %+v", keys)
	}
}

func TestCRUDObjectStorePipelineRuns(t *testing.T) {
	path := tempFile()
	db, err := New(path, 200)
	if err != nil {
		t.Fatal(err)
	}
	defer os.Remove(path)

	namespace := models.NewNamespace("test_namespace", "Test Namespace", "Testing namespace")

	err = db.InsertNamespace(namespace)
	if err != nil {
		t.Fatal(err)
	}

	pipelineConfig := sdk.NewPipeline("test_pipeline", "Test Pipeline").WithTasks([]sdk.Task{
		*sdk.NewTask("test_task", "task:latest").WithDependsOnOne("test_task_depends", sdk.RequiredParentStatusAny),
	}).WithTriggers([]sdk.PipelineTriggerConfig{
		{
			Name:  "test_trigger",
			Label: "test_trigger_label",
			Settings: map[string]string{
				"test_setting_key": "test_setting_value",
			},
		},
	})
	pipeline := models.NewPipeline(namespace.ID, pipelineConfig)

	err = db.InsertPipeline(pipeline)
	if err != nil {
		t.Fatal(err)
	}

	run := models.NewRun(namespace.ID, pipeline.ID, models.TriggerInfo{
		Name:  "test_trigger_name",
		Label: "test_trigger_label",
	}, []models.Variable{})
	run.TaskRuns = []string{
		"test_task_run",
	}

	runID, err := db.InsertRun(run)
	if err != nil {
		t.Fatal(err)
	}
	run.ID = runID

	key := models.NewObjectStoreKey("test_key")

	err = db.InsertObjectStoreRunKey("test_namespace", "test_pipeline", runID, key)
	if err != nil {
		t.Fatal(err)
	}

	keys, err := db.ListObjectStoreRunKeys("test_namespace", "test_pipeline", runID)
	if err != nil {
		t.Fatal(err)
	}

	if diff := cmp.Diff(*key, keys[0]); diff != "" {
		t.Errorf("unexpected map values (-want +got):\n%s", diff)
	}

	err = db.DeleteObjectStoreRunKey("test_namespace", "test_pipeline", runID, key.Key)
	if err != nil {
		t.Fatal(err)
	}

	keys, err = db.ListObjectStoreRunKeys("test_namespace", "test_pipeline", runID)
	if err != nil {
		t.Fatal(err)
	}

	if len(keys) != 0 {
		t.Fatalf("expected 0 keys but found keys: %+v", keys)
	}
}

func TestCRUDSecretStorePipelineKeys(t *testing.T) {
	path := tempFile()
	db, err := New(path, 200)
	if err != nil {
		t.Fatal(err)
	}
	defer os.Remove(path)

	namespace := models.NewNamespace("test_namespace", "Test Namespace", "Testing namespace")

	err = db.InsertNamespace(namespace)
	if err != nil {
		t.Fatal(err)
	}

	pipelineConfig := sdk.NewPipeline("test_pipeline", "Test Pipeline").WithTasks([]sdk.Task{
		*sdk.NewTask("test_task", "task:latest").WithDependsOnOne("test_task_depends", sdk.RequiredParentStatusAny),
	}).WithTriggers([]sdk.PipelineTriggerConfig{
		{
			Name:  "test_trigger",
			Label: "test_trigger_label",
			Settings: map[string]string{
				"test_setting_key": "test_setting_value",
			},
		},
	})
	pipeline := models.NewPipeline(namespace.ID, pipelineConfig)

	err = db.InsertPipeline(pipeline)
	if err != nil {
		t.Fatal(err)
	}

	key := models.NewSecretStoreKey("test_key")

	err = db.InsertSecretStorePipelineKey("test_namespace", "test_pipeline", key, false)
	if err != nil {
		t.Fatal(err)
	}

	keys, err := db.ListSecretStorePipelineKeys("test_namespace", "test_pipeline")
	if err != nil {
		t.Fatal(err)
	}

	if diff := cmp.Diff(*key, keys[0]); diff != "" {
		t.Errorf("unexpected map values (-want +got):\n%s", diff)
	}

	fetchedKey, err := db.GetSecretStorePipelineKey("test_namespace", "test_pipeline", key.Key)
	if err != nil {
		t.Fatal(err)
	}

	if diff := cmp.Diff(*key, fetchedKey); diff != "" {
		t.Errorf("unexpected map values (-want +got):\n%s", diff)
	}

	err = db.DeleteSecretStorePipelineKey("test_namespace", "test_pipeline", key.Key)
	if err != nil {
		t.Fatal(err)
	}

	keys, err = db.ListSecretStorePipelineKeys("test_namespace", "test_pipeline")
	if err != nil {
		t.Fatal(err)
	}

	if len(keys) != 0 {
		t.Fatalf("expected 0 keys but found keys: %+v", keys)
	}
}

func TestCRUDSecretStoreGlobalKeys(t *testing.T) {
	path := tempFile()
	db, err := New(path, 200)
	if err != nil {
		t.Fatal(err)
	}
	defer os.Remove(path)

	key := models.NewSecretStoreKey("test_key")

	err = db.InsertSecretStoreGlobalKey(key, false)
	if err != nil {
		t.Fatal(err)
	}

	keys, err := db.ListSecretStoreGlobalKeys()
	if err != nil {
		t.Fatal(err)
	}

	if diff := cmp.Diff(*key, keys[0]); diff != "" {
		t.Errorf("unexpected map values (-want +got):\n%s", diff)
	}

	fetchedKey, err := db.GetSecretStoreGlobalKey(key.Key)
	if err != nil {
		t.Fatal(err)
	}

	if diff := cmp.Diff(*key, fetchedKey); diff != "" {
		t.Errorf("unexpected map values (-want +got):\n%s", diff)
	}

	err = db.DeleteSecretStoreGlobalKey(key.Key)
	if err != nil {
		t.Fatal(err)
	}

	keys, err = db.ListSecretStoreGlobalKeys()
	if err != nil {
		t.Fatal(err)
	}

	if len(keys) != 0 {
		t.Fatalf("expected 0 keys but found keys: %+v", keys)
	}
}
