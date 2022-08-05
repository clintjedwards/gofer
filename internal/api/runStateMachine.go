package api

import (
	"bufio"
	"fmt"
	"os"
	"sync/atomic"
	"time"

	"github.com/clintjedwards/gofer/internal/scheduler"
	"github.com/clintjedwards/gofer/internal/storage"
	"github.com/clintjedwards/gofer/internal/syncmap"
	"github.com/clintjedwards/gofer/models"
	"github.com/rs/zerolog/log"
)

// Used to keep track of a run as it progresses through the necessary states.
type RunStateMachine struct {
	API      *API
	Pipeline *models.Pipeline
	Run      *models.Run
	TaskRuns syncmap.Syncmap[string, models.TaskRun]
	StopRuns *atomic.Bool // Used to stop the progression of a run
}

func NewRunStateMachine(pipeline *models.Pipeline, run *models.Run) *RunStateMachine {
	var stopRuns atomic.Bool
	stopRuns.Store(false)

	return &RunStateMachine{
		Pipeline: pipeline,
		Run:      run,
		TaskRuns: syncmap.New[string, models.TaskRun](),
		StopRuns: &stopRuns,
	}
}

// Mark a task run object as finished.
func (r *RunStateMachine) setTaskRunFinished(id string, code *int64,
	status models.TaskRunStatus, reason *models.TaskRunStatusReason,
) error {
	taskRun, exists := r.TaskRuns.Get(id)
	if !exists {
		return fmt.Errorf("could not find task run")
	}

	taskRun.State = models.TaskRunStateComplete
	taskRun.Status = status

	r.TaskRuns.Set(id, taskRun)

	err := r.API.db.UpdateTaskRun(&taskRun, storage.UpdatableTaskRunFields{
		ExitCode:     code,
		Status:       &status,
		State:        ptr(models.TaskRunStateComplete),
		Ended:        ptr(time.Now().UnixMilli()),
		StatusReason: reason,
	})
	if err != nil {
		return err
	}

	go r.API.events.Publish(models.EventCompletedTaskRun{
		NamespaceID: taskRun.Namespace,
		PipelineID:  taskRun.Pipeline,
		RunID:       taskRun.Run,
		TaskRunID:   taskRun.ID,
		Status:      taskRun.Status,
	})

	return nil
}

func (r *RunStateMachine) setRunFinished(status models.RunStatus, reason *models.RunStatusReason) error {
	err := r.API.db.UpdateRun(r.Pipeline.Namespace, r.Pipeline.ID, r.Run.ID, storage.UpdatableRunFields{
		State:        ptr(models.RunStateComplete),
		Status:       &status,
		StatusReason: reason,
		Ended:        ptr(time.Now().UnixMilli()),
	})
	if err != nil {
		return err
	}

	go r.API.events.Publish(models.EventCompletedRun{
		NamespaceID: r.Run.Namespace,
		PipelineID:  r.Run.Pipeline,
		RunID:       r.Run.ID,
		Status:      r.Run.Status,
	})

	return nil
}

func (r *RunStateMachine) setTaskRunState(taskRun models.TaskRun, state models.TaskRunState) error {
	err := r.API.db.UpdateTaskRun(&taskRun, storage.UpdatableTaskRunFields{
		State: &state,
	})
	if err != nil {
		return err
	}

	taskRun.State = state

	r.TaskRuns.Set(taskRun.ID, taskRun)

	return nil
}

// executeTaskTree creates all downstream task runs for a particular run. After creating all task runs it
// then blocks and monitors the run until it is finished.
func (r *RunStateMachine) executeTaskTree() {
	// Launch per-run clean up jobs.
	go r.handleRunObjectExpiry()
	go r.handleRunLogExpiry()

	// Launch a new task run for each task found.
	for _, task := range r.Pipeline.Tasks {
		go r.launchTaskRun(task)
	}

	// Finally monitor the entire run until it finishes. This will block until the run has ended.
	r.waitRunFinish()
}

// Check a dependency tree to see if all parent tasks have been finished.
func (r *RunStateMachine) parentTaskFinished(dependencies *map[string]models.RequiredParentStatus) bool {
	for parent := range *dependencies {
		parentTaskRun, exists := r.TaskRuns.Get(parent)
		if !exists {
			return false
		}

		if parentTaskRun.State != models.TaskRunStateComplete {
			return false
		}
	}

	return true
}

func (r *RunStateMachine) parallelismLimitExceeded() bool {
	limit := r.Pipeline.Parallelism

	if limit == 0 && r.API.config.RunParallelismLimit == 0 {
		return false
	}

	if limit > int64(r.API.config.RunParallelismLimit) {
		limit = int64(r.API.config.RunParallelismLimit)
	}

	runs, err := r.API.db.ListRuns(nil, 0, 0, r.Pipeline.Namespace, r.Pipeline.ID)
	if err != nil {
		return true
	}

	var runsInProgress int64 = 0

	for _, run := range runs {
		if run.State != models.RunStateComplete {
			runsInProgress++
		}
	}

	return runsInProgress >= limit
}

// Check a dependency tree to see if all parent tasks are in the correct states.
func (r *RunStateMachine) taskDependenciesSatisfied(dependencies *map[string]models.RequiredParentStatus) error {
	for parent, requiredStatus := range *dependencies {
		parentTaskRun, exists := r.TaskRuns.Get(parent)
		if !exists {
			return fmt.Errorf("could not find parent dependency for task dependencies satisfied function")
		}

		switch requiredStatus {
		case models.RequiredParentStatusUnknown:
			return fmt.Errorf("a parent dependency should never be in the state Unknown")
		case models.RequiredParentStatusAny:
			if parentTaskRun.Status != models.TaskRunStatusSuccessful &&
				parentTaskRun.Status != models.TaskRunStatusFailed &&
				parentTaskRun.Status != models.TaskRunStatusSkipped {
				return fmt.Errorf("parent %s has incorrect status %s for required 'any' dependency", parent,
					parentTaskRun.Status)
			}
		case models.RequiredParentStatusSuccess:
			if parentTaskRun.Status != models.TaskRunStatusSuccessful {
				return fmt.Errorf("parent %s has incorrect status %s for required 'successful' dependency", parent,
					parentTaskRun.Status)
			}
		case models.RequiredParentStatusFailure:
			if parentTaskRun.Status != models.TaskRunStatusFailed {
				return fmt.Errorf("parent %s has incorrect status %s for required 'failed' dependency", parent,
					parentTaskRun.Status)
			}
		}
	}

	return nil
}

// Monitors all task run statuses and determines the final run status based on all
// finished task runs. It will block until all task runs have finished.
func (r *RunStateMachine) waitRunFinish() {
	err := r.API.db.UpdateRun(r.Pipeline.Namespace, r.Pipeline.ID, r.Run.ID, storage.UpdatableRunFields{
		State: ptr(models.RunStateRunning),
	})
	if err != nil {
		log.Error().Err(err).Msg("storage error occurred while waiting for run to finish")
		return
	}

	// If the task run map hasn't had all the entries com in we should wait until it does.
	for {
		if len(r.TaskRuns.Keys()) != len(r.Pipeline.Tasks) {
			time.Sleep(time.Millisecond * 500)
			continue
		}

		break
	}

	// We loop over the task runs to make sure all are complete.
outerLoop:
	for {
		time.Sleep(time.Millisecond * 500)
		for _, id := range r.TaskRuns.Keys() {
			taskRun, exists := r.TaskRuns.Get(id)
			if !exists {
				continue outerLoop
			}

			if taskRun.State != models.TaskRunStateComplete {
				continue outerLoop
			}
		}

		break
	}

	// When all are finished we now need to get a final tallying of what the run's result is.
	// A run is only successful if all task_runs were successful. If any task_run is in an
	// unknown or failed state we fail the run, if any task_run is cancelled we mark the run as cancelled.
	for _, id := range r.TaskRuns.Keys() {
		taskRun, exists := r.TaskRuns.Get(id)
		if !exists {
			log.Error().Err(err).Msg("somehow we couldn't get a task run in the task run map while evaluating finished task runs. This should never happen.")
			return
		}

		switch taskRun.Status {
		case models.TaskRunStatusUnknown:
			fallthrough
		case models.TaskRunStatusFailed:
			_ = r.setRunFinished(models.RunStatusFailed, &models.RunStatusReason{
				Reason:      models.RunStatusReasonKindAbnormalExit,
				Description: "One or more task runs failed during execution",
			})
			return
		case models.TaskRunStatusSuccessful:
			continue
		case models.TaskRunStatusCancelled:
			_ = r.setRunFinished(models.RunStatusCancelled, &models.RunStatusReason{
				Reason:      models.RunStatusReasonKindAbnormalExit,
				Description: "One or more task runs were cancelled during execution",
			})
			return

		case models.TaskRunStatusSkipped:
			continue
		}
	}

	_ = r.setRunFinished(models.RunStatusSuccessful, nil)
}

// Monitors all task run statuses and determines the final run status based on all
// finished task runs. It will block until all task runs have finished.
func (r *RunStateMachine) waitTaskRunFinish(schedulerID, taskRunID string) error {
	for {
		response, err := r.API.scheduler.GetState(scheduler.GetStateRequest{
			SchedulerID: schedulerID,
		})
		if err != nil {
			_ = r.setTaskRunFinished(taskRunID, nil, models.TaskRunStatusUnknown, &models.TaskRunStatusReason{
				Reason:      models.TaskRunStatusReasonKindSchedulerError,
				Description: fmt.Sprintf("Could not query the scheduler for task run state; %v", err),
			})
			return err
		}

		switch response.State {
		case scheduler.ContainerStateUnknown:
			_ = r.setTaskRunFinished(taskRunID, nil, models.TaskRunStatusUnknown, &models.TaskRunStatusReason{
				Reason:      models.TaskRunStatusReasonKindSchedulerError,
				Description: "An unknown error has occurred on the scheduler level; This should never happen",
			})
			return nil
		case scheduler.ContainerStateRunning:
			fallthrough
		case scheduler.ContainerStatePaused:
			time.Sleep(time.Millisecond * 500)
			continue
		case scheduler.ContainerStateCancelled:
			_ = r.setTaskRunFinished(taskRunID, nil, models.TaskRunStatusCancelled, &models.TaskRunStatusReason{
				Reason:      models.TaskRunStatusReasonKindCancelled,
				Description: "A user has cancelled the task run",
			})
			return nil
		case scheduler.ContainerStateExited:
			if response.ExitCode == 0 {
				_ = r.setTaskRunFinished(taskRunID, &response.ExitCode, models.TaskRunStatusSuccessful, nil)
				return nil
			}

			_ = r.setTaskRunFinished(taskRunID, &response.ExitCode, models.TaskRunStatusFailed,
				&models.TaskRunStatusReason{
					Reason:      models.TaskRunStatusReasonKindAbnormalExit,
					Description: "Task run exited with abnormal exit code.",
				})
			return nil

		default:
			_ = r.setTaskRunFinished(taskRunID, nil, models.TaskRunStatusUnknown, &models.TaskRunStatusReason{
				Reason:      models.TaskRunStatusReasonKindSchedulerError,
				Description: fmt.Sprintf("Could not query the scheduler for task run state; %v", err),
			})
			return nil
		}
	}
}

// Tracks state and log progress of a task_run. It automatically updates the provided task-run
// with the resulting state change(s). This function will block until the task-run has
// reached a terminal state.
func (r *RunStateMachine) monitorTaskRun(schedulerID, taskRunID string) error {
	go r.handleLogUpdates(schedulerID, taskRunID)
	err := r.waitTaskRunFinish(schedulerID, taskRunID)
	if err != nil {
		return err
	}

	return nil
}

func (r *RunStateMachine) handleLogUpdates(schedulerID, taskRunID string) {
	taskRun, exists := r.TaskRuns.Get(taskRunID)
	if !exists {
		log.Error().Msg("Could not find task run in run state machine")
		return
	}

	logReader, err := r.API.scheduler.GetLogs(scheduler.GetLogsRequest{
		SchedulerID: schedulerID,
	})
	if err != nil {
		log.Error().Err(err).Msg("Scheduler error; could not get logs")
		return
	}

	logFile, err := os.Create(taskRunLogFilePath(r.API.config.TaskRunLogsDir, taskRun.Namespace,
		taskRun.Pipeline, taskRun.Run, taskRun.ID))
	if err != nil {
		log.Error().Err(err).Msg("Could not open task run log file for writing")
		return
	}

	scanner := bufio.NewScanner(logReader)
	for scanner.Scan() {
		_, _ = logFile.WriteString(scanner.Text() + "\n")
	}

	// When the reader is finished we place a special marker to signify that this file is finished with.
	// This allows other readers of the file within Gofer to know the difference between a file that is still being
	// written to and a file that will not be written to any further.
	_, _ = logFile.WriteString(GOFEREOF)

	logFile.Close()

	err = scanner.Err()
	if err != nil {
		log.Error().Err(err).Msg("Could not properly read from logging stream")
	}
}

// Removes run level object_store objects once a run is past it's expiry threshold.
func (r *RunStateMachine) handleRunObjectExpiry() {
	limit := r.API.config.ObjectStore.RunObjectExpiry

	// We ask for the limit of runs plus one extra
	runs, err := r.API.db.ListRuns(nil, 0, limit+1, r.Pipeline.Namespace, r.Pipeline.ID)
	if err != nil {
		log.Error().Err(err).Msg("could not get runs for run expiry processing; db error")
		return
	}

	// If there aren't enough runs to reach the limit there is nothing to remove
	if limit > len(runs) {
		return
	}

	if len(runs) == 0 {
		return
	}

	expiredRun := runs[len(runs)-1]

	// If the run is still in progress wait for it to be done
	for expiredRun.State != models.RunStateComplete {
		time.Sleep(time.Second)

		expiredRun, err = r.API.db.GetRun(r.Pipeline.Namespace, r.Pipeline.ID, expiredRun.ID)
		if err != nil {
			log.Error().Err(err).Msg("could not get runs for run expiry processing; db error")
			return
		}
	}

	if expiredRun.StoreObjectsExpired {
		return
	}

	objectKeys, err := r.API.db.ListObjectStoreRunKeys(r.Pipeline.Namespace, r.Pipeline.ID, expiredRun.ID)
	if err != nil {
		log.Error().Err(err).Msg("could not get runs for run expiry processing; db error")
		return
	}

	for _, key := range objectKeys {
		// Delete it from the object store
		err = r.API.objectStore.DeleteObject(runObjectKey(r.Pipeline.Namespace, r.Pipeline.ID, expiredRun.ID, key.Key))
		if err != nil {
			log.Error().Err(err).Msg("could not delete run object for run expiry processing; db error")
			continue
		}

		// Delete it from the run's records
		err = r.API.db.DeleteObjectStoreRunKey(r.Pipeline.Namespace, r.Pipeline.ID, expiredRun.ID, key.Key)
		if err != nil {
			log.Error().Err(err).Msg("could not delete run object for run expiry processing; db error")
			continue
		}
	}

	err = r.API.db.UpdateRun(r.Pipeline.Namespace, r.Pipeline.ID, expiredRun.ID, storage.UpdatableRunFields{
		StoreObjectsExpired: ptr(true),
	})
	if err != nil {
		log.Error().Err(err).Msg("could not get runs for run expiry processing; db error")
		return
	}

	r.API.events.Publish(models.EventExpiredRunObjects{
		NamespaceID: r.Pipeline.Namespace,
		PipelineID:  r.Pipeline.ID,
		RunID:       expiredRun.ID,
	})
}

func (r *RunStateMachine) handleRunLogExpiry() {
	limit := r.API.config.RunLogExpiry

	// We ask for the limit of runs plus one extra
	runs, err := r.API.db.ListRuns(nil, 0, limit+1, r.Pipeline.Namespace, r.Pipeline.ID)
	if err != nil {
		log.Error().Err(err).Msg("could not get runs for run log expiry processing; db error")
		return
	}

	// If there aren't enough runs to reach the limit there is nothing to remove
	if limit > len(runs) {
		return
	}

	if len(runs) == 0 {
		return
	}

	expiredRun := runs[len(runs)-1]

	// If the run is still in progress wait for it to be done
	for expiredRun.State != models.RunStateComplete {
		time.Sleep(time.Second)

		expiredRun, err = r.API.db.GetRun(r.Pipeline.Namespace, r.Pipeline.ID, expiredRun.ID)
		if err != nil {
			log.Error().Err(err).Msg("could not get runs for run log expiry processing; db error")
			return
		}
	}

	var taskRuns []models.TaskRun

	// If the task runs are in progress we wait for it to be done.
outerLoop:
	for {
		taskRuns, err = r.API.db.ListTaskRuns(0, 0, r.Pipeline.Namespace, r.Pipeline.ID, expiredRun.ID)
		if err != nil {
			log.Error().Err(err).Msg("could not get task runs for run log expiry processing; db error")
			return
		}

		for _, taskRun := range taskRuns {
			if taskRun.State != models.TaskRunStateComplete {
				time.Sleep(time.Millisecond * 500)
				continue outerLoop
			}
		}

		break
	}

	removedFiles := []string{}
	for _, taskRun := range taskRuns {
		taskRun := taskRun
		if taskRun.LogsExpired || taskRun.LogsRemoved {
			continue
		}

		logFilePath := taskRunLogFilePath(r.API.config.TaskRunLogsDir, taskRun.Namespace,
			taskRun.Pipeline, taskRun.Run, taskRun.ID)

		err := os.Remove(logFilePath)
		if err != nil {
			log.Debug().Err(err).Msg("could not remove task run log file")
		}

		err = r.API.db.UpdateTaskRun(&taskRun, storage.UpdatableTaskRunFields{
			LogsExpired: ptr(true),
			LogsRemoved: ptr(true),
		})
		if err != nil {
			log.Error().Err(err).Msg("could not update task run state; db error")
			continue
		}

		removedFiles = append(removedFiles, logFilePath)
	}

	log.Debug().Strs("removed_files", removedFiles).Msg("removed task run logs")
}

// Launches a brand new task run as part of a larger run for a specific task.
// It blocks until the task run has completed.
func (r *RunStateMachine) launchTaskRun(task models.Task) {
	// Start by created a new task run and saving it to the state machine and disk.
	newTaskRun := models.NewTaskRun(r.Pipeline.Namespace, r.Pipeline.ID, r.Run.ID, task)

	r.TaskRuns.Set(newTaskRun.ID, *newTaskRun)

	err := r.API.db.InsertTaskRun(newTaskRun)
	if err != nil {
		log.Error().Err(err).Msg("could not register task run; db error")
		return
	}

	// Alert the event bus that a new task run is being started.
	go r.API.events.Publish(models.EventCreatedTaskRun{
		NamespaceID: r.Pipeline.Namespace,
		PipelineID:  r.Pipeline.ID,
		RunID:       r.Run.ID,
		TaskRunID:   newTaskRun.ID,
	})

	envVars := combineVariables(r.Run, &task)

	// Determine the task run's final variable set and pass them in.
	err = r.API.db.UpdateTaskRun(newTaskRun, storage.UpdatableTaskRunFields{
		Variables: &envVars,
	})
	if err != nil {
		log.Error().Err(err).Msg("could not launch task run; db error")
		return
	}

	// Now we examine the validity of the task run to be started and wait for it's dependents to finish running.
	err = r.setTaskRunState(*newTaskRun, models.TaskRunStateWaiting)
	if err != nil {
		log.Error().Err(err).Msg("could not launch task run; db error")
		return
	}

	// First we need to make sure all the parents of the current task are in a finished state.
	for !r.parentTaskFinished(&newTaskRun.DependsOn) {
		time.Sleep(time.Millisecond * 500)
	}

	err = r.setTaskRunState(*newTaskRun, models.TaskRunStateProcessing)
	if err != nil {
		log.Error().Err(err).Msg("could not launch task run; db error")
		return
	}

	// Then check to make sure that the parents all finished in the required states. If not
	// we'll have to mark this task as skipped.
	err = r.taskDependenciesSatisfied(&newTaskRun.DependsOn)
	if err != nil {
		_ = r.setTaskRunFinished(newTaskRun.ID, nil, models.TaskRunStatusSkipped, &models.TaskRunStatusReason{
			Reason:      models.TaskRunStatusReasonKindFailedPrecondition,
			Description: fmt.Sprintf("Task could not be run due to unmet dependencies; %v", err),
		})
		return
	}

	// After this point we're sure the task is in a state to be run. So we attempt to
	// contact the scheduler and start the container.

	// First we attempt to find any object/secret store variables and replace them
	// with the correct var. At first glance this may seem like a task that can move upwards
	// but it's important that this run only after a task's parents have already run
	// this enables users to be sure that one task can pass variables to other downstream tasks.

	// We create a copy of variables so that we can substitute in secrets and objects.
	// to eventually pass them into the start container function.
	envVars, err = r.API.interpolateVars(r.Pipeline.Namespace, r.Pipeline.ID, &r.Run.ID, envVars)
	if err != nil {
		_ = r.setTaskRunFinished(newTaskRun.ID, nil, models.TaskRunStatusFailed, &models.TaskRunStatusReason{
			Reason:      models.TaskRunStatusReasonKindFailedPrecondition,
			Description: fmt.Sprintf("Task could not be run due to inability to retrieve interpolated variables; %v", err),
		})
		return
	}

	preparedEnvVars := map[string]string{}
	for _, variable := range envVars {
		preparedEnvVars[variable.Key] = variable.Value
	}

	containerName := taskContainerID(r.Pipeline.Namespace, r.Pipeline.ID, r.Run.ID, newTaskRun.ID)

	response, err := r.API.scheduler.StartContainer(scheduler.StartContainerRequest{
		ID:               containerName,
		ImageName:        newTaskRun.Image,
		EnvVars:          preparedEnvVars,
		RegistryAuth:     newTaskRun.RegistryAuth,
		AlwaysPull:       false,
		EnableNetworking: false,
		Entrypoint:       newTaskRun.Entrypoint,
		Command:          newTaskRun.Command,
	})
	if err != nil {
		_ = r.setTaskRunFinished(newTaskRun.ID, nil, models.TaskRunStatusFailed, &models.TaskRunStatusReason{
			Reason:      models.TaskRunStatusReasonKindSchedulerError,
			Description: fmt.Sprintf("Task could not be run due to inability to be scheduled; %v", err),
		})
		return
	}

	err = r.API.db.UpdateTaskRun(newTaskRun, storage.UpdatableTaskRunFields{
		State:   ptr(models.TaskRunStateRunning),
		Started: ptr(time.Now().UnixMilli()),
	})
	if err != nil {
		log.Error().Err(err).Msg("could not launch task run; db error")
		return
	}

	go r.API.events.Publish(models.EventStartedTaskRun{
		NamespaceID: r.Pipeline.Namespace,
		PipelineID:  r.Pipeline.ID,
		RunID:       r.Run.ID,
		TaskRunID:   newTaskRun.ID,
	})

	newTaskRun.State = models.TaskRunStateRunning
	r.TaskRuns.Set(newTaskRun.ID, *newTaskRun)

	// Block until task run is finished and log results.
	err = r.monitorTaskRun(response.SchedulerID, newTaskRun.ID)
	if err != nil {
		log.Error().Err(err).Msg("could not launch task run; db error")
		return
	}
}
