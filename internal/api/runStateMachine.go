package api

import (
	"bufio"
	"encoding/json"
	"fmt"
	"os"
	"sync/atomic"
	"time"

	"github.com/clintjedwards/gofer/events"
	"github.com/clintjedwards/gofer/internal/models"
	"github.com/clintjedwards/gofer/internal/scheduler"
	"github.com/clintjedwards/gofer/internal/storage"
	"github.com/clintjedwards/gofer/internal/syncmap"
	"github.com/rs/zerolog/log"
)

// Used to keep track of a run as it progresses through the necessary states.
type RunStateMachine struct {
	API      *API
	Pipeline *models.PipelineMetadata
	Config   *models.PipelineConfig
	Run      *models.Run
	TaskRuns syncmap.Syncmap[string, models.TaskRun]
	StopRuns *atomic.Bool // Used to stop the progression of a run
}

func (api *API) newRunStateMachine(pipeline *models.PipelineMetadata, config *models.PipelineConfig, run *models.Run) *RunStateMachine {
	var stopRuns atomic.Bool
	stopRuns.Store(false)

	return &RunStateMachine{
		API:      api,
		Pipeline: pipeline,
		Config:   config,
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

	err := r.API.db.UpdatePipelineTaskRun(r.API.db, taskRun.Namespace, taskRun.Pipeline, taskRun.Run, taskRun.ID,
		storage.UpdatablePipelineTaskRunFields{
			ExitCode:     code,
			Status:       ptr(string(status)),
			State:        ptr(string(models.TaskRunStateComplete)),
			Ended:        ptr(time.Now().UnixMilli()),
			StatusReason: ptr(reason.ToJSON()),
		})
	if err != nil {
		return err
	}

	go r.API.events.Publish(events.EventTaskRunCompleted{
		NamespaceID: taskRun.Namespace,
		PipelineID:  taskRun.Pipeline,
		RunID:       taskRun.Run,
		TaskRunID:   taskRun.ID,
		Status:      string(taskRun.Status),
	})

	return nil
}

func (r *RunStateMachine) setRunFinished(status models.RunStatus, reason *models.RunStatusReason) error {
	err := r.API.db.UpdatePipelineRun(r.API.db, r.Pipeline.Namespace, r.Pipeline.ID, r.Run.ID, storage.UpdatablePipelineRunFields{
		State:        ptr(string(models.RunStateComplete)),
		Status:       ptr(string(status)),
		StatusReason: ptr(reason.ToJSON()),
		Ended:        ptr(time.Now().UnixMilli()),
	})
	if err != nil {
		return err
	}

	go r.API.events.Publish(events.EventRunCompleted{
		NamespaceID:   r.Run.Namespace,
		PipelineID:    r.Run.Pipeline,
		RunID:         r.Run.ID,
		Status:        string(status),
		InitiatorType: string(r.Run.Initiator.Type),
		InitiatorName: r.Run.Initiator.Name,
	})

	return nil
}

func (r *RunStateMachine) setTaskRunState(taskRun models.TaskRun, state models.TaskRunState) error {
	err := r.API.db.UpdatePipelineTaskRun(r.API.db, r.Pipeline.Namespace, r.Pipeline.ID, r.Run.ID, taskRun.ID,
		storage.UpdatablePipelineTaskRunFields{
			State: ptr(string(state)),
		})
	if err != nil {
		return err
	}

	taskRun.State = state

	r.TaskRuns.Set(taskRun.ID, taskRun)

	return nil
}

// Creates the auto-injected token for Gofer's `InjectAPIToken` feature.
// This simply evaluates whether the token will be needed in the future and then sets it to be
// injected when the tasks that need it are eventually run.
//
// Gofer can auto create client tokens and inject them into the environment for tasks in a run.
// This is a convenience function so that tasks can easily talk to the Gofer CLI.
func (r *RunStateMachine) createAutoInjectToken() {
	// Check we actually need to do this
	createToken := false

	for _, task := range r.Config.CustomTasks {
		if task.InjectAPIToken {
			createToken = true
			break
		}
	}

	for _, task := range r.Config.CommonTasks {
		if task.InjectAPIToken {
			createToken = true
			break
		}
	}

	if createToken {
		token, hash := r.API.createNewAPIToken()
		newToken := models.NewToken(hash, models.TokenKindClient, []string{r.Pipeline.Namespace}, map[string]string{
			"description": "This token was automatically created by Gofer API at the user's request. Visit https://clintjedwards.com/gofer/ref/pipeline_configuration/index.html#auto-inject-api-tokens to learn more.",
		}, time.Hour*48)

		_, err := r.API.db.InsertToken(r.API.db, newToken.ToStorage())
		if err != nil {
			log.Error().Err(err).Msg("could not save token to storage")
		}

		err = r.API.secretStore.PutSecret(
			pipelineSecretKey(r.Pipeline.Namespace, r.Pipeline.ID, fmt.Sprintf("gofer_api_token_%d", r.Run.ID)), token, true)
		if err != nil {
			log.Error().Err(err).Msg("could not save token to storage")
		}
	}
}

// executeTaskTree creates all downstream task runs for a particular run. After creating all task runs it
// then blocks and monitors the run until it is finished.
func (r *RunStateMachine) executeTaskTree() {
	// Launch per-run clean up jobs.
	go r.handleRunObjectExpiry()
	go r.handleRunLogExpiry()

	r.createAutoInjectToken()

	// Launch a new task run for each task found.
	for _, task := range r.Config.CustomTasks {
		task := task
		go r.launchTaskRun(&task, true)
	}

	for _, taskSettings := range r.Config.CommonTasks {
		// We create a half filled model of common task so that
		// we can pass it to the next step where it will get fully filled in.
		// We only do this because the next step already has the facilities to handle
		// a task run failure properly.
		task := models.CommonTask{
			Settings: taskSettings,
		}

		go r.launchTaskRun(&task, true)
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
	limit := r.Config.Parallelism

	if limit == 0 && r.API.config.RunParallelismLimit == 0 {
		return false
	}

	if limit > int64(r.API.config.RunParallelismLimit) {
		limit = int64(r.API.config.RunParallelismLimit)
	}

	if limit == 0 {
		return false
	}

	runsRaw, err := r.API.db.ListPipelineRuns(r.API.db, 0, 0, r.Pipeline.Namespace, r.Pipeline.ID)
	if err != nil {
		return true
	}

	var runsInProgress int64

	for _, runRaw := range runsRaw {
		var run models.Run
		run.FromStorage(&runRaw)
		if run.State != models.RunStateComplete {
			runsInProgress++
		}
	}

	if runsInProgress >= limit {
		log.Debug().Int64("run_id", r.Run.ID).Int64("limit", limit).
			Int64("currently_in_progress", runsInProgress).
			Msg("parallelism limit exceeded; waiting for runs to end before launching new run")
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
	err := r.API.db.UpdatePipelineRun(r.API.db, r.Pipeline.Namespace, r.Pipeline.ID, r.Run.ID, storage.UpdatablePipelineRunFields{
		State: ptr(string(models.RunStateRunning)),
	})
	if err != nil {
		log.Error().Err(err).Msg("storage error occurred while waiting for run to finish")
		return
	}

	// If the task run map hasn't had all the entries com in we should wait until it does.
	for {
		if len(r.TaskRuns.Keys()) != len(r.Config.CustomTasks)+len(r.Config.CommonTasks) {
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
			err := r.setRunFinished(models.RunStatusFailed, &models.RunStatusReason{
				Reason:      models.RunStatusReasonKindAbnormalExit,
				Description: "One or more task runs failed during execution",
			})
			if err != nil {
				log.Error().Err(err).Msg("could not set run finished")
			}

			return
		case models.TaskRunStatusSuccessful:
			continue
		case models.TaskRunStatusCancelled:
			err := r.setRunFinished(models.RunStatusCancelled, &models.RunStatusReason{
				Reason:      models.RunStatusReasonKindAbnormalExit,
				Description: "One or more task runs were cancelled during execution",
			})
			if err != nil {
				log.Error().Err(err).Msg("could not set run finished")
			}
			return

		case models.TaskRunStatusSkipped:
			continue
		}
	}

	_ = r.setRunFinished(models.RunStatusSuccessful, nil)
}

// Monitors all task run statuses and determines the final run status based on all
// finished task runs. It will block until all task runs have finished.
func (r *RunStateMachine) waitTaskRunFinish(containerID, taskRunID string) error {
	for {
		response, err := r.API.scheduler.GetState(scheduler.GetStateRequest{
			ID: containerID,
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
func (r *RunStateMachine) monitorTaskRun(containerID, taskRunID string) error {
	go r.handleLogUpdates(containerID, taskRunID)
	err := r.waitTaskRunFinish(containerID, taskRunID)
	if err != nil {
		return err
	}

	return nil
}

func (r *RunStateMachine) handleLogUpdates(containerID, taskRunID string) {
	taskRun, exists := r.TaskRuns.Get(taskRunID)
	if !exists {
		log.Error().Msg("Could not find task run in run state machine")
		return
	}

	logReader, err := r.API.scheduler.GetLogs(scheduler.GetLogsRequest{
		ID: containerID,
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
	runs, err := r.API.db.ListPipelineRuns(r.API.db, 0, limit+1, r.Pipeline.Namespace, r.Pipeline.ID)
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

	expiredRunRaw := runs[len(runs)-1]
	var expiredRun models.Run
	expiredRun.FromStorage(&expiredRunRaw)

	// If the run is still in progress wait for it to be done
	for expiredRun.State != models.RunStateComplete {
		time.Sleep(time.Second)

		expiredRunRaw, err = r.API.db.GetPipelineRun(r.API.db, r.Pipeline.Namespace, r.Pipeline.ID, expiredRun.ID)
		if err != nil {
			log.Error().Err(err).Msg("could not get runs for run expiry processing; db error")
			return
		}

		var tmpExpiredRun models.Run
		tmpExpiredRun.FromStorage(&expiredRunRaw)
		expiredRun = tmpExpiredRun
	}

	if expiredRun.StoreObjectsExpired {
		return
	}

	objectKeys, err := r.API.db.ListObjectStoreRunKeys(r.API.db, r.Pipeline.Namespace, r.Pipeline.ID, expiredRun.ID)
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
		err = r.API.db.DeleteObjectStoreRunKey(r.API.db, r.Pipeline.Namespace, r.Pipeline.ID, expiredRun.ID, key.Key)
		if err != nil {
			log.Error().Err(err).Msg("could not delete run object for run expiry processing; db error")
			continue
		}
	}

	err = r.API.db.UpdatePipelineRun(r.API.db, r.Pipeline.Namespace, r.Pipeline.ID, expiredRun.ID,
		storage.UpdatablePipelineRunFields{
			StoreObjectsExpired: ptr(true),
		})
	if err != nil {
		log.Error().Err(err).Msg("could not get runs for run expiry processing; db error")
		return
	}

	r.API.events.Publish(events.EventRunObjectsExpired{
		NamespaceID: r.Pipeline.Namespace,
		PipelineID:  r.Pipeline.ID,
		RunID:       expiredRun.ID,
	})
}

func (r *RunStateMachine) handleRunLogExpiry() {
	limit := r.API.config.TaskRunLogExpiry

	// We ask for the limit of runs plus one extra
	runs, err := r.API.db.ListPipelineRuns(r.API.db, 0, limit+1, r.Pipeline.Namespace, r.Pipeline.ID)
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

	expiredRunRaw := runs[len(runs)-1]
	var expiredRun models.Run
	expiredRun.FromStorage(&expiredRunRaw)

	// If the run is still in progress wait for it to be done
	for expiredRun.State != models.RunStateComplete {
		time.Sleep(time.Second)

		expiredRunRaw, err = r.API.db.GetPipelineRun(r.API.db, r.Pipeline.Namespace, r.Pipeline.ID, expiredRun.ID)
		if err != nil {
			log.Error().Err(err).Msg("could not get runs for run log expiry processing; db error")
			return
		}

		var tmpExpiredRun models.Run
		tmpExpiredRun.FromStorage(&expiredRunRaw)
		expiredRun = tmpExpiredRun
	}

	var taskRuns []models.TaskRun

	// If the task runs are in progress we wait for it to be done.
outerLoop:
	for {
		taskRunsRaw, err := r.API.db.ListPipelineTaskRuns(r.API.db, 0, 0, r.Pipeline.Namespace, r.Pipeline.ID, expiredRun.ID)
		if err != nil {
			log.Error().Err(err).Msg("could not get task runs for run log expiry processing; db error")
			return
		}

		for _, taskRunRaw := range taskRunsRaw {
			var taskRun models.TaskRun
			taskRun.FromStorage(&taskRunRaw)
			taskRuns = append(taskRuns, taskRun)
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

		err = r.API.db.UpdatePipelineTaskRun(r.API.db, taskRun.Namespace, taskRun.Pipeline, taskRun.Run, taskRun.ID,
			storage.UpdatablePipelineTaskRunFields{
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

// Registers[^1] and Launches a brand new task run as part of a larger run for a specific task.
// It blocks until the task run has completed.
//
// [^1]: The register parameter controls whether the task is registered in the database, announces it's creation
// via events. It's useful to turn this off when we're trying to revive a taskRun that is previously lost.
func (r *RunStateMachine) launchTaskRun(task models.Task, register bool) {
	kind := models.TaskKindCustom

	// If the task is a common task we need to check that it is in the registry, fill in those registry details,
	// and then fail properly if it is not.
	commonTask, isCommonTask := task.(*models.CommonTask)
	if isCommonTask {
		kind = models.TaskKindCommon
		registration, exists := r.API.commonTasks.Get(commonTask.Settings.Name)
		if !exists {
			newTaskRun := models.NewTaskRun(r.Pipeline.Namespace, r.Pipeline.ID, r.Config.Version, r.Run.ID, kind, task)

			r.TaskRuns.Set(newTaskRun.ID, *newTaskRun)

			err := r.API.db.InsertPipelineTaskRun(r.API.db, newTaskRun.ToStorage())
			if err != nil {
				log.Error().Err(err).Msg("could not register task run; db error")
				return
			}

			// Alert the event bus that a new task run is being started.
			go r.API.events.Publish(events.EventTaskRunCreated{
				NamespaceID: r.Pipeline.Namespace,
				PipelineID:  r.Pipeline.ID,
				RunID:       r.Run.ID,
				TaskRunID:   newTaskRun.ID,
			})

			_ = r.setTaskRunFinished(newTaskRun.ID, nil, models.TaskRunStatusFailed, &models.TaskRunStatusReason{
				Reason:      models.TaskRunStatusReasonKindFailedPrecondition,
				Description: "Common Task was not found in Gofer registry.",
			})
			return
		}

		commonTask.Registration = *registration
		task = commonTask
	}

	// Start by created a new task run and saving it to the state machine and disk.
	newTaskRun := models.NewTaskRun(r.Pipeline.Namespace, r.Pipeline.ID, r.Config.Version, r.Run.ID, kind, task)

	r.TaskRuns.Set(newTaskRun.ID, *newTaskRun)

	if register {
		err := r.API.db.InsertPipelineTaskRun(r.API.db, newTaskRun.ToStorage())
		if err != nil {
			log.Error().Err(err).Msg("could not register task run; db error")
			return
		}

		// Alert the event bus that a new task run is being started.
		go r.API.events.Publish(events.EventTaskRunCreated{
			NamespaceID: r.Pipeline.Namespace,
			PipelineID:  r.Pipeline.ID,
			RunID:       r.Run.ID,
			TaskRunID:   newTaskRun.ID,
		})
	}

	envVars := combineVariables(r.Run, task)

	envVarsJSON, err := json.Marshal(envVars)
	if err != nil {
		log.Error().Err(err).Msg("could not register task run; db error")
		return
	}

	// Determine the task run's final variable set and pass them in.
	err = r.API.db.UpdatePipelineTaskRun(r.API.db, newTaskRun.Namespace, newTaskRun.Pipeline, newTaskRun.Run, newTaskRun.ID,
		storage.UpdatablePipelineTaskRunFields{
			Variables: ptr(string(envVarsJSON)),
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
	for !r.parentTaskFinished(ptr(newTaskRun.Task.GetDependsOn())) {
		time.Sleep(time.Millisecond * 500)
	}

	err = r.setTaskRunState(*newTaskRun, models.TaskRunStateProcessing)
	if err != nil {
		log.Error().Err(err).Msg("could not launch task run; db error")
		return
	}

	// Then check to make sure that the parents all finished in the required states. If not
	// we'll have to mark this task as skipped.
	err = r.taskDependenciesSatisfied(ptr(newTaskRun.Task.GetDependsOn()))
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
		err := r.setTaskRunFinished(newTaskRun.ID, nil, models.TaskRunStatusFailed, &models.TaskRunStatusReason{
			Reason:      models.TaskRunStatusReasonKindFailedPrecondition,
			Description: fmt.Sprintf("Task could not be run due to inability to retrieve interpolated variables; %v", err),
		})
		if err != nil {
			log.Error().Err(err).Msg("could not properly set task run")
		}
		return
	}

	preparedEnvVars := map[string]string{}
	for _, variable := range envVars {
		preparedEnvVars[variable.Key] = variable.Value
	}

	containerName := taskContainerID(r.Pipeline.Namespace, r.Pipeline.ID, r.Run.ID, newTaskRun.ID)

	_, err = r.API.scheduler.StartContainer(scheduler.StartContainerRequest{
		ID:           containerName,
		ImageName:    newTaskRun.Task.GetImage(),
		EnvVars:      preparedEnvVars,
		RegistryAuth: newTaskRun.Task.GetRegistryAuth(),
		AlwaysPull:   false,
		Networking:   nil,
		Entrypoint:   newTaskRun.Task.GetEntrypoint(),
		Command:      newTaskRun.Task.GetCommand(),
	})
	if err != nil {
		_ = r.setTaskRunFinished(newTaskRun.ID, nil, models.TaskRunStatusFailed, &models.TaskRunStatusReason{
			Reason:      models.TaskRunStatusReasonKindSchedulerError,
			Description: fmt.Sprintf("Task could not be run due to inability to be scheduled; %v", err),
		})
		return
	}

	err = r.API.db.UpdatePipelineTaskRun(r.API.db, newTaskRun.Namespace, newTaskRun.Pipeline, newTaskRun.Run, newTaskRun.ID,
		storage.UpdatablePipelineTaskRunFields{
			State:   ptr(string(models.TaskRunStateRunning)),
			Started: ptr(time.Now().UnixMilli()),
		})
	if err != nil {
		log.Error().Err(err).Msg("could not launch task run; db error")
		return
	}

	go r.API.events.Publish(events.EventTaskRunStarted{
		NamespaceID: r.Pipeline.Namespace,
		PipelineID:  r.Pipeline.ID,
		RunID:       r.Run.ID,
		TaskRunID:   newTaskRun.ID,
	})

	newTaskRun.State = models.TaskRunStateRunning
	r.TaskRuns.Set(newTaskRun.ID, *newTaskRun)

	containerID := taskContainerID(r.Pipeline.Namespace, r.Pipeline.ID, r.Run.ID, newTaskRun.ID)

	// Block until task run is finished and log results.
	err = r.monitorTaskRun(containerID, newTaskRun.ID)
	if err != nil {
		log.Error().Err(err).Msg("could not launch task run; db error")
		return
	}
}
