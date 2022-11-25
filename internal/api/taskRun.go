package api

import (
	"time"

	"github.com/clintjedwards/gofer/internal/models"
	"github.com/clintjedwards/gofer/internal/scheduler"
)

// cancelTaskRun calls upon the scheduler to terminate a specific container. The urgency of this request is
// controlled by the force parameter. Normally scheduler will simply send a SIGTERM and wait for a
// graceful exit and on force they will instead send a SIGKILL.
// The associated timeout controls how long the containers are waited upon until they are sent a SIGKILL.
func (api *API) cancelTaskRun(taskRun *models.TaskRun, force bool) error {
	timeout := api.config.TaskRunStopTimeout

	if force {
		timeout = time.Millisecond * 500
	}

	containerID := taskContainerID(taskRun.Namespace, taskRun.Pipeline, taskRun.Run, taskRun.ID)

	err := api.scheduler.StopContainer(scheduler.StopContainerRequest{
		ID:      containerID,
		Timeout: timeout,
	})
	if err != nil {
		return err
	}

	return nil
}
