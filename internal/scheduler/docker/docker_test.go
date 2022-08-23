package docker

import (
	"testing"
	"time"

	"github.com/clintjedwards/gofer/internal/scheduler"
)

func TestStartContainer(t *testing.T) {
	orch, err := New(false, time.Second)
	if err != nil {
		t.Fatal(err)
	}

	containerID := "test_container_name"

	_, err = orch.StartContainer(scheduler.StartContainerRequest{
		ID:        containerID,
		ImageName: "ubuntu:latest",
		Command:   &[]string{"sleep", "2s"},
	})
	if err != nil {
		t.Fatal(err)
	}

	time.Sleep(time.Second)

	resp, err := orch.GetState(scheduler.GetStateRequest{
		ID: containerID,
	})
	if err != nil {
		t.Fatal(err)
	}

	if resp.State != scheduler.ContainerStateRunning {
		t.Fatalf("container in incorrect state; should be %s; found %s", scheduler.ContainerStateRunning, resp.State)
	}
}
