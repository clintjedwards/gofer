package storage

import (
	"errors"
	"os"
	"testing"

	"github.com/google/go-cmp/cmp"
)

func TestCRUDPipelineDeployments(t *testing.T) {
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

	deployment := PipelineDeployment{
		Namespace:    "test_namespace",
		Pipeline:     "test_pipeline",
		ID:           0,
		StartVersion: 0,
		EndVersion:   1,
		Started:      "0",
		Ended:        "0",
		State:        "RUNNING",
		Status:       "ACTIVE",
		StatusReason: "REASON",
		Logs:         "LOGS",
	}

	err = db.InsertPipelineDeployment(db, &deployment)
	if err != nil {
		t.Fatal(err)
	}

	deployments, err := db.ListPipelineDeployments(db, 0, 0, pipeline.Namespace, pipeline.ID)
	if err != nil {
		t.Fatal(err)
	}

	if len(deployments) != 1 {
		t.Errorf("expected 1 element in list found %d", len(deployments))
	}

	if diff := cmp.Diff(deployment, deployments[0]); diff != "" {
		t.Errorf("unexpected map values (-want +got):\n%s", diff)
	}

	deployments, err = db.ListRunningPipelineDeployments(db, 0, 0, pipeline.Namespace, pipeline.ID)
	if err != nil {
		t.Fatal(err)
	}

	if len(deployments) != 1 {
		t.Errorf("expected 1 element in list found %d", len(deployments))
	}

	if diff := cmp.Diff(deployment, deployments[0]); diff != "" {
		t.Errorf("unexpected map values (-want +got):\n%s", diff)
	}

	fetchedDeployment, err := db.GetPipelineDeployment(db, namespace.ID, pipeline.ID, deployment.ID)
	if err != nil {
		t.Fatal(err)
	}

	if diff := cmp.Diff(deployment, fetchedDeployment); diff != "" {
		t.Errorf("unexpected map values (-want +got):\n%s", diff)
	}

	fetchedDeployment.Ended = "4"
	fetchedDeployment.State = "DISABLED"
	fetchedDeployment.Status = "UPDATED_STATUS"
	fetchedDeployment.StatusReason = "UPDATED_STATUS_REASON"
	fetchedDeployment.Logs = "UPDATED_LOGS"

	err = db.UpdatePipelineDeployment(db, namespace.ID, pipeline.ID, deployment.ID,
		UpdatablePipelineDeploymentFields{
			Ended:        &fetchedDeployment.Ended,
			State:        &fetchedDeployment.State,
			Status:       &fetchedDeployment.Status,
			StatusReason: &fetchedDeployment.StatusReason,
			Logs:         &fetchedDeployment.Logs,
		})
	if err != nil {
		t.Fatal(err)
	}

	updatedDeployment, err := db.GetPipelineDeployment(db, namespace.ID, pipeline.ID, deployment.ID)
	if err != nil {
		t.Fatal(err)
	}

	if diff := cmp.Diff(fetchedDeployment, updatedDeployment); diff != "" {
		t.Errorf("unexpected map values (-want +got):\n%s", diff)
	}

	err = db.DeletePipelineDeployment(db, namespace.ID, pipeline.ID, deployment.ID)
	if err != nil {
		t.Fatal(err)
	}

	_, err = db.GetPipelineDeployment(db, namespace.ID, pipeline.ID, deployment.ID)
	if !errors.Is(err, ErrEntityNotFound) {
		t.Fatal("expected error Not Found; found alternate error")
	}
}
