package api

import (
	"bufio"
	"errors"
	"fmt"
	"os"
	"strconv"
	"strings"
	"sync"
	"time"

	"github.com/clintjedwards/gofer/internal/models"
	"github.com/clintjedwards/gofer/internal/scheduler"
	"github.com/clintjedwards/gofer/internal/storage"
	"github.com/clintjedwards/gofer/internal/syncmap"
	"github.com/rs/zerolog/log"
)

const (
	TASKCONTAINERIDFORMAT = "%s_%d_%s" // pipelineid_runid_taskrunid

	// GOFEREOF is a special string marker we include at the end of log files.
	// It denotes that no further logs will be written. This is to provide the functionality for downstream
	// applications to follow log files and not also have to monitor the container for state to know when
	// logs will no longer be printed.
	GOFEREOF string = "GOFER_EOF"
)

// mergeMaps combines many string maps in a "last one in wins" format. Meaning that in case of key collision
// the last map to be added will overwrite the value of the previous key.
func mergeMaps[KeyType comparable, ValueType any](maps ...map[KeyType]ValueType) map[KeyType]ValueType {
	newMap := map[KeyType]ValueType{}

	for _, extraMap := range maps {
		for key, value := range extraMap {
			newMap[key] = value
		}
	}

	return newMap
}

// startTaskRun starts a specific task run and updates the taskrun with either a failed or running state once
// complete. This function updates the taskRun provided to it automatically.
func (api *API) startTaskRun(sc scheduler.StartContainerRequest, taskRun *models.TaskRun) (string, error) {
	containerInfo, err := api.scheduler.StartContainer(sc)
	if err != nil {
		taskRun.SetFinishedAbnormal(models.ContainerStateFailed,
			models.TaskRunFailure{
				Kind:        models.TaskRunFailureKindSchedulerError,
				Description: fmt.Sprintf("Could not start container on scheduler: %v", err),
			},
			1)

		storageErr := api.storage.UpdateTaskRun(storage.UpdateTaskRunRequest{TaskRun: taskRun})
		if storageErr != nil {
			log.Error().Err(err).Msg("could not update run")
		}

		if containerInfo.SchedulerID != "" {
			go api.handleLogUpdates(containerInfo.SchedulerID, taskRun)
		}

		return "", err
	}

	taskRun.State = models.ContainerStateRunning
	taskRun.SchedulerID = containerInfo.SchedulerID
	taskRun.Started = time.Now().UnixMilli()
	err = api.storage.UpdateTaskRun(storage.UpdateTaskRunRequest{TaskRun: taskRun})
	if err != nil {
		return "", err
	}

	return containerInfo.SchedulerID, nil
}

// monitorTaskRun attaches state and log monitoring goroutines to track taskrun state and progress over time.
// It automatically updates the provided taskrun with the resulting state change(s).
// This function will block until the taskrun has reached a finished state.
func (api *API) monitorTaskRun(schedulerID string, taskRun *models.TaskRun) error {
	go api.handleLogUpdates(schedulerID, taskRun)
	err := api.waitTaskRunFinish(schedulerID, taskRun)
	if err != nil {
		log.Error().Err(err).Str("task", taskRun.ID).
			Str("pipeline", taskRun.PipelineID).
			Int64("run", taskRun.RunID).Msg("could not get state for container update")
	}

	err = api.storage.UpdateTaskRun(storage.UpdateTaskRunRequest{TaskRun: taskRun})
	if err != nil {
		log.Error().Err(err).Str("task", taskRun.ID).
			Str("pipeline", taskRun.PipelineID).
			Int64("run", taskRun.RunID).Msg("could not update task run state due to storage err")
		return err
	}

	return nil
}

// waitTaskRunFinish monitors the task run's container for many possible states. Depending on the state
// it will update the task run with that specific state and then exit. Until it reaches a terminal state
// this function will block.
func (api *API) waitTaskRunFinish(schedulerID string, taskRun *models.TaskRun) error {
	for {
		state, err := api.scheduler.GetState(scheduler.GetStateRequest{
			SchedulerID: schedulerID,
		})
		if err != nil {
			taskRun.SetFinishedAbnormal(models.ContainerStateFailed,
				models.TaskRunFailure{
					Kind:        models.TaskRunFailureKindSchedulerError,
					Description: fmt.Sprintf("Could not query the scheduler for container state: %v", err),
				},
				1)
			return err
		}

		switch state.State {
		case models.ContainerStateRunning, models.ContainerStateProcessing, models.ContainerStateWaiting:
			time.Sleep(time.Second * 5)
			continue
		case models.ContainerStateSuccess:
			taskRun.SetFinishedSuccess()
			return nil
		case models.ContainerStateCancelled:
			taskRun.SetFinishedAbnormal(models.ContainerStateCancelled,
				models.TaskRunFailure{
					Kind:        models.TaskRunFailureKindCancelled,
					Description: "Task cancelled during run.",
				},
				1)
			return nil
		case models.ContainerStateFailed:
			taskRun.SetFinishedAbnormal(models.ContainerStateFailed,
				models.TaskRunFailure{
					Kind:        models.TaskRunFailureKindAbnormalExit,
					Description: "Container exited with abnormal exit code.",
				},
				state.ExitCode)
			return nil
		default:
			taskRun.SetFinishedAbnormal(models.ContainerStateUnknown,
				models.TaskRunFailure{
					Kind:        models.TaskRunFailureKindUnknown,
					Description: "An unknown error has occurred. This should never happen.",
				},
				state.ExitCode)
			return nil
		}
	}
}

// handleLogUpdates monitors for and stores logs for a given run. If run again for a particular log file it will
// truncate previous logs and repopulate logs with logs from scheduler.
func (api *API) handleLogUpdates(schedulerID string, currentTaskRun *models.TaskRun) {
	logReader, err := api.scheduler.GetLogs(scheduler.GetLogsRequest{
		SchedulerID: schedulerID,
	})
	if err != nil {
		log.Error().Err(err).Msg("Scheduler error; could not get logs")
		return
	}

	logfile, err := os.Create(api.taskRunLogFilePath(currentTaskRun))
	if err != nil {
		log.Error().Err(err).Msg("Could not open task run log file for writing")
		return
	}

	scanner := bufio.NewScanner(logReader)
	for scanner.Scan() {
		_, _ = logfile.WriteString(scanner.Text() + "\n")
	}

	// When the reader is finished we place a special marker to signify that this file is finished with.
	// This allows other readers of the file within Gofer to know the difference between a file that is still being
	// written to and a file that will not be written to any further.
	_, _ = logfile.WriteString(GOFEREOF)

	logfile.Close()

	err = scanner.Err()
	if err != nil {
		log.Error().Err(err).Msg("Could not properly read from logging stream")
	}
}

func (api *API) taskRunLogFilePath(taskRun *models.TaskRun) string {
	const TASKRUNFILEPATH = "%s/%s_%d_%s" // folder/pipelineid_runid_taskrunid

	return fmt.Sprintf(TASKRUNFILEPATH, api.config.TaskRunLogsDir,
		taskRun.PipelineID, taskRun.RunID, taskRun.ID)
}

// interpolateVars takes in a map of mixed plaintext and raw secret/store strings and populates it with the fetched
// results.
// We do pipeline level store substitutions and secret substitutions only here because those are globally available.
// Run level interpolation is written in another function because it's run dependent.
func (api *API) interpolateVars(namespace, pipeline string, mixedMap map[string]string) (map[string]string, error) {
	parsedMap := map[string]string{}

	for mapKey, value := range mixedMap {
		// Parse for secrets
		key := parseInterpolationSyntax("secret", value) // secret{{ example }}
		if key != value {
			secret, err := api.secretStore.GetSecret(secretKey(namespace, pipeline, key))
			if err != nil {
				return nil, fmt.Errorf("could not find secret %q in secret store", key)
			}
			parsedMap[mapKey] = secret
			continue
		}

		// Parse for pipeline objects
		key = parseInterpolationSyntax("pipeline", value) // pipeline{{ example }}
		if key != value {
			object, err := api.objectStore.GetObject(pipelineObjectKey(namespace, pipeline, key))
			if err != nil {
				return nil, fmt.Errorf("could not find pipeline object %q in object store", key)
			}
			parsedMap[mapKey] = string(object)
			continue
		}

		parsedMap[mapKey] = value
		continue
	}

	return parsedMap, nil
}

// interpolateRunStoreVars takes in a map of mixed plaintext and raw run store strings and populates it with the fetched
// results.
func (api *API) interpolateRunStoreVars(namespace, pipeline string, mixedMap map[string]string, run int64) (map[string]string, error) {
	parsedMap := map[string]string{}

	for mapKey, value := range mixedMap {
		// Parse for run objects
		key := parseInterpolationSyntax("run", value) // run{{ example }}
		if key != value {
			object, err := api.objectStore.GetObject(runObjectKey(namespace, pipeline, key, run))
			if err != nil {
				return nil, fmt.Errorf("could not find run object %q in object store", key)
			}
			parsedMap[mapKey] = string(object)
			continue
		}

		parsedMap[mapKey] = value
		continue
	}

	return parsedMap, nil
}

// parseInterpolationSyntax checks a string for the existence of a interpolation format. ex: "secret{{ example }}".
// If it is we return that key without the brackets, if it is not, the string is returned.
//
// Currently the supported interpolation syntaxes are:
// `secret{{ example }}` for inserting from the secret store.
// `pipeline{{ example }}` for inserting from the pipeline object store.
// `run{{ example }}` for inserting from the run object store.
func parseInterpolationSyntax(prefix, variable string) string {
	variable = strings.TrimSpace(variable)
	if strings.HasPrefix(variable, fmt.Sprintf("%s{{", prefix)) && strings.HasSuffix(variable, "}}") {
		variable = strings.TrimPrefix(variable, fmt.Sprintf("%s{{", prefix))
		variable = strings.TrimSuffix(variable, "}}")
		return strings.TrimSpace(variable)
	}
	return variable
}

// reviveLostTaskRun attempts to re-run as taskrun that has somehow been orphaned. It is used for taskruns
// that have not been scheduled yet, but will be after other task runs have finished.
func (api *API) reviveLostTaskRun(taskStatusMap *syncmap.Syncmap[string, models.ContainerState], taskrun *models.TaskRun) {
	taskStatusMap.Set(taskrun.Task.ID, taskrun.State)
	api.events.Publish(models.NewEventStartedTaskRun(*taskrun))

	// First check to make sure all the parents of the current task are in a finished state.
	for {
		if !parentTasksFinished(taskStatusMap, taskrun.DependsOn) {
			time.Sleep(time.Millisecond * 500)
			continue
		}

		break
	}

	// Then check to make sure that the parents all finished in the required states. If not
	// we'll have to cancel this task.
	if err := dependenciesSatisfied(taskStatusMap, taskrun.DependsOn); err != nil {
		taskrun.SetFinishedAbnormal(models.ContainerStateSkipped,
			models.TaskRunFailure{
				Kind:        models.TaskRunFailureKindFailedPrecondition,
				Description: fmt.Sprintf("Task could not be run due to unmet dependencies: %v", err),
			}, 1)

		err := api.storage.UpdateTaskRun(storage.UpdateTaskRunRequest{TaskRun: taskrun})
		if err != nil {
			log.Error().Err(err).Msg("could not update task run")
		}
		taskStatusMap.Set(taskrun.Task.ID, models.ContainerStateSkipped)
		api.events.Publish(models.NewEventCompletedTaskRun(*taskrun))
		return
	}

	// First we attempt to find any pipeline/secret store variables and replace them with the correct var.
	parsedEnvVars, err := api.interpolateVars(taskrun.NamespaceID, taskrun.PipelineID, taskrun.EnvVars)
	if err != nil {
		taskrun.SetFinishedAbnormal(models.ContainerStateFailed, models.TaskRunFailure{
			Kind:        models.TaskRunFailureKindFailedPrecondition,
			Description: fmt.Sprintf("Task could not be run due to unmet dependencies; could not find one or more keys: %v", err),
		}, 1)
		err := api.storage.UpdateTaskRun(storage.UpdateTaskRunRequest{TaskRun: taskrun})
		if err != nil {
			log.Error().Err(err).Msg("could not update task run")
		}
		taskStatusMap.Set(taskrun.Task.ID, taskrun.State)
		api.events.Publish(models.NewEventCompletedTaskRun(*taskrun))
		return
	}

	// Then we attempt to find any run store variables and replace them with the correct var.
	parsedEnvVars, err = api.interpolateRunStoreVars(taskrun.NamespaceID, taskrun.PipelineID, parsedEnvVars, taskrun.RunID)
	if err != nil {
		taskrun.SetFinishedAbnormal(models.ContainerStateFailed, models.TaskRunFailure{
			Kind:        models.TaskRunFailureKindFailedPrecondition,
			Description: fmt.Sprintf("Task could not be run due to unmet dependencies; could not find one or more keys: %v", err),
		}, 1)
		err := api.storage.UpdateTaskRun(storage.UpdateTaskRunRequest{TaskRun: taskrun})
		if err != nil {
			log.Error().Err(err).Msg("could not update task run")
		}
		taskStatusMap.Set(taskrun.Task.ID, taskrun.State)
		api.events.Publish(models.NewEventCompletedTaskRun(*taskrun))
		return
	}

	// We create a run level token here that we pass into each task run. This allows task runs to perform actions
	// against the API.
	key, tokenObject, err := api.createNewAPIToken(models.TokenKindClient, []string{taskrun.NamespaceID},
		map[string]string{"description": "temporary run token"})
	if err != nil {
		log.Error().Err(err).Msg("could not create token")
	} else {
		defer api.storage.DeleteToken(storage.DeleteTokenRequest{
			Hash: tokenObject.Hash,
		})
	}

	taskrun.Secrets = map[string]string{
		"GOFER_API_TOKEN": key, // We use this token to give task runs the ability to interact with Gofer.
	}

	schedulerID, err := api.startTaskRun(scheduler.StartContainerRequest{
		ID:        fmt.Sprintf(TASKCONTAINERIDFORMAT, taskrun.PipelineID, taskrun.RunID, taskrun.ID),
		ImageName: taskrun.Image,
		EnvVars:   mergeMaps(taskrun.Secrets, parsedEnvVars),
		Exec: scheduler.Exec{
			Shell:  taskrun.Exec.Shell,
			Script: taskrun.Exec.Script,
		},
		RegistryUser: taskrun.RegistryAuth.User,
		RegistryPass: parseInterpolationSyntax("secret", taskrun.RegistryAuth.Pass),
	}, taskrun)
	if err != nil {
		log.Error().Err(err).Str("id", taskrun.ID).
			Str("pipeline", taskrun.PipelineID).Int64("run", taskrun.RunID).Msg("task run could not be started")
		taskStatusMap.Set(taskrun.Task.ID, taskrun.State)
		api.events.Publish(models.NewEventCompletedTaskRun(*taskrun))
		return
	}
	log.Info().Str("id", taskrun.ID).Str("pipeline", taskrun.PipelineID).
		Int64("run", taskrun.RunID).Msg("started task run")

	api.events.Publish(models.NewEventScheduledTaskRun(*taskrun))
	taskStatusMap.Set(taskrun.Task.ID, taskrun.State)

	err = api.monitorTaskRun(schedulerID, taskrun)
	if err != nil {
		log.Error().Err(err).Str("id", taskrun.ID).
			Str("pipeline", taskrun.PipelineID).Int64("run", taskrun.RunID).
			Msg("task run monitor encountered an error")
		taskStatusMap.Set(taskrun.Task.ID, taskrun.State)
		api.events.Publish(models.NewEventCompletedTaskRun(*taskrun))
		return
	}

	taskStatusMap.Set(taskrun.Task.ID, taskrun.State)
	api.events.Publish(models.NewEventCompletedTaskRun(*taskrun))

	log.Info().Str("id", taskrun.ID).Str("status", string(taskrun.State)).Msg("finished task run")
}

// createNewTaskRun launches a brand new task run as part of a larger run for a specific task.
// It blocks until the taskrun has gone through the full lifecycle or waiting, running, and then finally
// is finished.
func (api *API) createNewTaskRun(taskStatusMap *syncmap.Syncmap[string, models.ContainerState], run models.Run, task models.Task, token string) {
	newTaskRun := models.NewTaskRun(run, task)
	api.events.Publish(models.NewEventStartedTaskRun(*newTaskRun))

	// These environment variables are present on every task run
	RunSpecificVars := map[string]string{
		"GOFER_PIPELINE_ID": run.PipelineID,
		"GOFER_RUN_ID":      strconv.Itoa(int(run.ID)),
		"GOFER_TASK_ID":     task.ID,
		"GOFER_TASK_IMAGE":  task.Image,
	}

	// We need to combine the environment variables we get from multiple sources in order to pass them finally to the
	// task. The order in which they are passed is very important as they can overwrite each other, even though the
	// intention of naming the environment variables are to prevent the chance of overwriting. The order in which they
	// are passed into the mergeMaps function determines the priority in reverse order. Last in the stack will overwrite
	// any conflicts from the others.
	//
	// 1) We first pass in the Gofer specific envvars as these are the most replaceable on the totem pole.
	// 2) We pass in the task specific envvars defined by the user in the pipeline config.
	// 3) Lastly we pass in the trigger's defined envvars, these are the most variable and most important since
	// they map back to the user's intent for a specific run.
	envVars := mergeMaps(RunSpecificVars, task.EnvVars, run.Variables)

	// We need to remove any envvars that have been added with an empty key
	for key := range envVars {
		key := strings.TrimSpace(key)
		if key == "" {
			delete(envVars, key)
		}
	}

	newTaskRun.EnvVars = envVars
	newTaskRun.Secrets = map[string]string{
		"GOFER_API_TOKEN": token, // We use this token to give task runs the ability to interact with Gofer via API.
	}
	newTaskRun.State = models.ContainerStateWaiting

	err := api.storage.AddTaskRun(storage.AddTaskRunRequest{TaskRun: newTaskRun})
	if err != nil {
		if errors.Is(err, storage.ErrEntityExists) {
			log.Error().Err(err).Msg("could not add task run")
			return
		}
		log.Error().Err(err).Msg("could not add task run")
		return
	}

	taskStatusMap.Set(newTaskRun.Task.ID, newTaskRun.State)

	// First check to make sure all the parents of the current task are in a finished state.
	for {
		if !parentTasksFinished(taskStatusMap, task.DependsOn) {
			time.Sleep(time.Millisecond * 500)
			continue
		}

		break
	}

	// Then check to make sure that the parents all finished in the required states. If not
	// we'll have to cancel this task.
	if err := dependenciesSatisfied(taskStatusMap, task.DependsOn); err != nil {
		newTaskRun.SetFinishedAbnormal(models.ContainerStateSkipped,
			models.TaskRunFailure{
				Kind:        models.TaskRunFailureKindFailedPrecondition,
				Description: fmt.Sprintf("Task could not be run due to unmet dependencies: %v", err),
			}, 1)

		err = api.storage.UpdateTaskRun(storage.UpdateTaskRunRequest{TaskRun: newTaskRun})
		if err != nil {
			log.Error().Err(err).Msg("could not update task run")
		}
		taskStatusMap.Set(newTaskRun.Task.ID, models.ContainerStateSkipped)
		api.events.Publish(models.NewEventCompletedTaskRun(*newTaskRun))
		return
	}

	// First we attempt to find any pipeline/secret store variables and replace them with the correct var.
	parsedEnvVars, err := api.interpolateVars(newTaskRun.NamespaceID, newTaskRun.PipelineID, newTaskRun.EnvVars)
	if err != nil {
		newTaskRun.SetFinishedAbnormal(models.ContainerStateFailed, models.TaskRunFailure{
			Kind: models.TaskRunFailureKindFailedPrecondition,
			Description: fmt.Sprintf("Task could not be run due to unmet dependencies; "+
				"could not find one or more keys in store: %v", err),
		}, 1)
		err := api.storage.UpdateTaskRun(storage.UpdateTaskRunRequest{TaskRun: newTaskRun})
		if err != nil {
			log.Error().Err(err).Msg("could not update task run")
		}
		taskStatusMap.Set(newTaskRun.Task.ID, newTaskRun.State)
		api.events.Publish(models.NewEventCompletedTaskRun(*newTaskRun))
		return
	}

	// Then we attempt to find any run store variables and replace them with the correct var.
	parsedEnvVars, err = api.interpolateRunStoreVars(newTaskRun.NamespaceID, newTaskRun.PipelineID, parsedEnvVars, newTaskRun.RunID)
	if err != nil {
		newTaskRun.SetFinishedAbnormal(models.ContainerStateFailed, models.TaskRunFailure{
			Kind: models.TaskRunFailureKindFailedPrecondition,
			Description: fmt.Sprintf("Task could not be run due to unmet dependencies; "+
				"could not find one or more keys in store: %v", err),
		}, 1)
		err := api.storage.UpdateTaskRun(storage.UpdateTaskRunRequest{TaskRun: newTaskRun})
		if err != nil {
			log.Error().Err(err).Msg("could not update task run")
		}
		taskStatusMap.Set(newTaskRun.Task.ID, newTaskRun.State)
		api.events.Publish(models.NewEventCompletedTaskRun(*newTaskRun))
		return
	}

	// Finally start the task run.
	schedulerID, err := api.startTaskRun(scheduler.StartContainerRequest{
		ID:        fmt.Sprintf(TASKCONTAINERIDFORMAT, newTaskRun.PipelineID, newTaskRun.RunID, newTaskRun.ID),
		ImageName: newTaskRun.Image,
		EnvVars:   mergeMaps(newTaskRun.Secrets, parsedEnvVars),
		Exec: scheduler.Exec{
			Shell:  newTaskRun.Exec.Shell,
			Script: newTaskRun.Exec.Script,
		},
		RegistryUser: newTaskRun.RegistryAuth.User,
		RegistryPass: parseInterpolationSyntax("secret", newTaskRun.RegistryAuth.Pass),
	}, newTaskRun)
	if err != nil {
		log.Error().Err(err).Str("id", newTaskRun.ID).Str("pipeline", newTaskRun.PipelineID).
			Int64("run", newTaskRun.RunID).Msg("task run could not be started")
		taskStatusMap.Set(newTaskRun.Task.ID, newTaskRun.State)
		api.events.Publish(models.NewEventCompletedTaskRun(*newTaskRun))
		return
	}
	log.Info().Str("id", newTaskRun.ID).Str("pipeline", newTaskRun.PipelineID).
		Int64("run", newTaskRun.RunID).Msg("started task run")

	api.events.Publish(models.NewEventScheduledTaskRun(*newTaskRun))
	taskStatusMap.Set(newTaskRun.Task.ID, newTaskRun.State)

	// Block until taskrun status can be logged
	err = api.monitorTaskRun(schedulerID, newTaskRun)
	if err != nil {
		log.Error().Err(err).Str("id", newTaskRun.ID).Str("pipeline", newTaskRun.PipelineID).
			Int64("run", newTaskRun.RunID).Msg("task run monitor encountered an error")
		taskStatusMap.Set(newTaskRun.Task.ID, newTaskRun.State)
		api.events.Publish(models.NewEventCompletedTaskRun(*newTaskRun))
		return
	}

	taskStatusMap.Set(newTaskRun.Task.ID, newTaskRun.State)
	api.events.Publish(models.NewEventCompletedTaskRun(*newTaskRun))

	log.Info().Str("id", newTaskRun.ID).Str("status", string(newTaskRun.State)).
		Msg("finished task run")
}

// parentTasksFinished checks to see if all parents dependencies are in a finished state.
func parentTasksFinished(statusMap *syncmap.Syncmap[string, models.ContainerState], dependencies map[string]models.RequiredParentState) bool {
	for parentTaskName := range dependencies {
		// Check to see if all parents exist
		parentStatus, exists := statusMap.Get(parentTaskName)
		if !exists {
			return false
		}

		// Check to see if parent is still running
		if parentStatus == models.ContainerStateRunning ||
			parentStatus == models.ContainerStateProcessing ||
			parentStatus == models.ContainerStateWaiting ||
			parentStatus == models.ContainerStateUnknown {
			return false
		}
	}

	return true
}

// dependenciesSatisfied examines the dependency map to make sure that all parents are in the correct states.
// If there is a parent not in the correct state we report that back to the caller via an error.
func dependenciesSatisfied(statusMap *syncmap.Syncmap[string, models.ContainerState], dependencies map[string]models.RequiredParentState) error {
	for parentTaskName, parentRequiredState := range dependencies {
		// Check to see if all parents exist
		parentStatus, exists := statusMap.Get(parentTaskName)
		if !exists {
			return fmt.Errorf("parent %q was not found in executed tasks but is required for task", parentTaskName)
		}

		switch parentRequiredState {
		case models.RequiredParentStateFail:
			if parentStatus != models.ContainerStateFailed {
				return fmt.Errorf("parent %q is in incorrect state %q; task requires it be in state 'FAILED'",
					parentTaskName, parentStatus)
			}
		case models.RequiredParentStateSuccess:
			if parentStatus != models.ContainerStateSuccess {
				return fmt.Errorf("parent %q is in incorrect state %q; task requires it be in state 'SUCCESS'",
					parentTaskName, parentStatus)
			}
		}
	}

	return nil
}

// addNotifiersAsTasks takes the notifier configs and converts them into tasks for a potential taskrun.
// We need to do several things which are important to get right in order:
// * We need all pipeline tasks to be dependencies of the notifier task
// * We need to figure out which pipeline tasks might be run in case the user is using the "only" feature, so we
// don't end up waiting on tasks that will never be run.
// * We need to pass in the user requested envvars and the main process envvars set in the config.
func (api *API) addNotifiersAsTasks(notifiers map[string]models.PipelineNotifierConfig, pipelineTasks map[string]models.Task,
	filter map[string]struct{},
) (map[string]models.Task, error) {
	tasks := map[string]models.Task{}
	dependsOn := map[string]models.RequiredParentState{}

	// Add all the pipeline tasks as dependencies for the notifier tasks.
	for taskID := range pipelineTasks {
		// If a filter exists don't add tasks that are not found in it.
		if len(filter) != 0 {
			_, exists := filter[taskID]
			if !exists {
				continue
			}
		}

		dependsOn[taskID] = models.RequiredParentStateAny
	}

	// Remove any notifiers from the dependency list.
	for label := range notifiers {
		delete(dependsOn, label)
	}

	for label, config := range notifiers {
		notifier, exists := api.notifiers.Get(config.Kind)
		if !exists {
			return nil, fmt.Errorf("notifier %q is not a registered notifier", config.Kind)
		}

		// We need to format all user envvars going into the notifier as GOFER_NOTIFIER_USER_CONFIG_<value>
		formattedUserEnvVars := map[string]string{}
		for key, value := range config.Config {
			formattedUserEnvVars[fmt.Sprintf("GOFER_NOTIFIER_USER_CONFIG_%s", strings.ToUpper(key))] = value
		}

		// We need to format all Gofer config envvars going into the notifier as GOFER_NOTIFIER_MAIN_CONFIG_<value>
		formattedMainEnvVars := map[string]string{}
		for key, value := range notifier.EnvVars {
			formattedMainEnvVars[fmt.Sprintf("GOFER_NOTIFIER_MAIN_CONFIG_%s", strings.ToUpper(key))] = value
		}

		tasks[label] = models.Task{
			ID:           label,
			Description:  "auto-generated notifier task",
			Image:        notifier.Image,
			RegistryAuth: notifier.RegistryAuth,
			DependsOn:    dependsOn,
			EnvVars:      formattedUserEnvVars,
			Secrets:      formattedMainEnvVars,
		}
	}

	return tasks, nil
}

// createNewRun starts a new run and launches the goroutines responsible for running tasks.
func (api *API) createNewRun(namespaceID, pipelineID, triggerKind, triggerName string,
	taskFilter map[string]struct{}, vars map[string]string,
) (*models.Run, error) {
	pipeline, err := api.storage.GetPipeline(storage.GetPipelineRequest{NamespaceID: namespaceID, ID: pipelineID})
	if err != nil {
		return nil, err
	}

	if pipeline.State != models.PipelineStateActive {
		return nil, ErrPipelineNotActive
	}

	if pipeline.Sequential && pipeline.LastRunID != 0 {
		latestRun, err := api.storage.GetRun(storage.GetRunRequest{
			NamespaceID: namespaceID,
			PipelineID:  pipelineID,
			ID:          pipeline.LastRunID,
		})
		if err != nil {
			return nil, fmt.Errorf("could not verify state of last run for pipeline in sequential mode: %v", err)
		}

		if !latestRun.IsComplete() {
			return nil, ErrPipelineRunsInProgress
		}
	}

	newRun := models.NewRun(pipelineID, pipeline.Namespace, triggerKind, triggerName, taskFilter, vars)

	err = api.storage.AddRun(storage.AddRunRequest{Run: newRun})
	if err != nil {
		if errors.Is(err, storage.ErrEntityExists) {
			return nil, storage.ErrEntityExists
		}
		log.Error().Err(err).Msg("could not add run")
		return nil, fmt.Errorf("could not add run; %w", err)
	}

	go api.events.Publish(models.NewEventStartedRun(*newRun)) // Publish that the run is currently in motion.
	go api.handleRunObjectExpiry(namespaceID, pipelineID)     // Run objects expire after a given amount of runs.
	go api.handleRunLogExpiry(namespaceID, pipelineID)        // Run logs expire after a given amount of runs.
	go api.executeTaskTree(newRun)                            // Launch a tree of goroutines to handle task run dependencies.

	return newRun, nil
}

// handleRunLogExpiry removes all task run logs older than a certain run count.
func (api *API) handleRunLogExpiry(namespaceID, pipelineID string) {
	limit := api.config.RunLogExpiry

	for {
		runs, err := api.storage.GetAllRuns(storage.GetAllRunsRequest{
			NamespaceID: namespaceID,
			PipelineID:  pipelineID,
			Offset:      0,
			Limit:       limit,
		})
		if err != nil {
			log.Error().Err(err).Msg("could not remove old run logs")
			return
		}

		if len(runs) < limit {
			return
		}

		run, err := api.storage.GetRun(storage.GetRunRequest{
			NamespaceID: namespaceID,
			ID:          runs[len(runs)-1].ID,
			PipelineID:  pipelineID,
		})
		if err != nil {
			log.Error().Err(err).Msg("could not remove old run logs")
			return
		}

		// Make sure the run isn't still being used before we clean up the objects.
		if !run.IsComplete() {
			time.Sleep(time.Second * 2)
			continue
		}

		taskRuns, err := api.storage.GetAllTaskRuns(storage.GetAllTaskRunsRequest{
			NamespaceID: namespaceID,
			PipelineID:  pipelineID,
			RunID:       run.ID,
		})
		if err != nil {
			log.Error().Err(err).Msg("could not remove old run logs")
			return
		}

		// Make sure the taskrun isn't still being used before we clean up the objects.
		stillRunning := false
		for _, taskRun := range taskRuns {
			if !taskRun.IsComplete() {
				stillRunning = true
			}
		}

		if stillRunning {
			continue
		}

		removedFiles := []string{}
		for _, taskRun := range taskRuns {
			taskRun := taskRun
			err := os.Remove(api.taskRunLogFilePath(taskRun))
			if err != nil {
				log.Debug().Err(err).Msg("could not remove task run log file")
			}
			taskRun.LogsExpired = true
			taskRun.LogsRemoved = true
			err = api.storage.UpdateTaskRun(storage.UpdateTaskRunRequest{TaskRun: taskRun})
			if err != nil {
				log.Error().Err(err).Msg("could not update run")
			}
			removedFiles = append(removedFiles, api.taskRunLogFilePath(taskRun))
		}

		log.Debug().Strs("removed_files", removedFiles).Int("run_age_limit", limit).Int64("run_id", run.ID).Msg("old run logs removed")
		return
	}
}

// handleRunObjectExpiry removes run level objectstore objects once a run is past is expiry threshold.
func (api *API) handleRunObjectExpiry(namespace, pipeline string) {
	limit := api.config.ObjectStore.RunObjectExpiry
	runs, err := api.storage.GetAllRuns(storage.GetAllRunsRequest{
		NamespaceID: namespace,
		PipelineID:  pipeline,
		Offset:      0,
		Limit:       limit,
	})
	if err != nil {
		log.Error().Err(err).Msg("could not remove old run objects")
		return
	}

	if len(runs) < limit {
		return
	}

	for {
		run, err := api.storage.GetRun(storage.GetRunRequest{
			NamespaceID: namespace,
			PipelineID:  pipeline,
			ID:          runs[len(runs)-1].ID,
		})
		if err != nil {
			log.Error().Err(err).Msg("could not remove old run objects")
			return
		}

		// Make sure the run isn't still being used before we clean up the objects.
		if run.State != models.RunFailed && run.State != models.RunSuccess && run.State != models.RunCancelled {
			time.Sleep(time.Second * 5)
			continue
		}

		for _, object := range run.Objects {
			_ = api.objectStore.DeleteObject(object)
		}

		run.ObjectsExpired = true

		err = api.storage.UpdateRun(storage.UpdateRunRequest{Run: run})
		if err != nil {
			log.Error().Err(err).Msg("could not update run")
		}

		log.Debug().Int("removed_objects", len(run.Objects)).Int("run_age_limit", limit).Int64("run_id", run.ID).Msg("old run objects removed")
		return
	}
}

// executeTaskTree creates all downstream task runs for a particular run. After creating all task runs it
// then blocks and monitors the run until it is finished.
func (api *API) executeTaskTree(run *models.Run) {
	pipeline, err := api.storage.GetPipeline(storage.GetPipelineRequest{
		NamespaceID: run.NamespaceID,
		ID:          run.PipelineID,
	})
	if err != nil {
		log.Error().Err(err).Msg("could not get pipeline in order to run task tree")
		return
	}

	// We create a run level token here that we pass into each task run. This allows task runs to perform actions
	// against the API.
	key, tokenObject, err := api.createNewAPIToken(models.TokenKindClient, []string{run.NamespaceID},
		map[string]string{"description": "temporary run token"})
	if err != nil {
		log.Error().Err(err).Msg("could not create token")
	} else {
		defer api.storage.DeleteToken(storage.DeleteTokenRequest{
			Hash: tokenObject.Hash,
		})
	}

	// We run notifiers as regular run tasks, so that they enjoy all the same perks.
	notifierTasks, err := api.addNotifiersAsTasks(pipeline.Notifiers, pipeline.Tasks, run.Only)
	if err != nil {
		log.Error().Err(err).Msg("could not properly create notifier tasks")
	}

	runnableTasks := mergeMaps(notifierTasks, pipeline.Tasks)

	taskStatusMap := syncmap.New[string, models.ContainerState]()

	for id, task := range runnableTasks {
		// If it already exists in the status map, skip it
		_, exists := taskStatusMap.Get(id)
		if exists {
			continue
		}

		// If run.Only is empty then we want to run everything so auto add anything we find
		if len(run.Only) == 0 {
			go api.createNewTaskRun(&taskStatusMap, *run, task, key)
			// Put an initial entry into taskstatusmap so the run monitor knows how many to wait on.
			taskStatusMap.Set(task.ID, models.ContainerStateProcessing)
			continue
		}

		// Else if run.Only is not empty then we want to be selective about the tasks we want to run
		// making sure that we can't end up in a position of which it is impossible to continue.
		_, exists = run.Only[id]
		if !exists {
			continue
		}

		for parentTaskName := range task.DependsOn {
			_, exists := run.Only[parentTaskName]
			if !exists {
				run.SetFailed("Precondition Failure", "A task to be executed depends on a task that isn't in the 'only' filter")
				err := api.storage.UpdateRun(storage.UpdateRunRequest{Run: run})
				if err != nil {
					log.Error().Err(err).Msg("could not update run")
				}
			}
		}

		go api.createNewTaskRun(&taskStatusMap, *run, task, key)
		// Put an initial entry into taskstatusmap so the run monitor knows how many to wait on.
		taskStatusMap.Set(task.ID, models.ContainerStateProcessing)
	}

	err = api.monitorRunStatus(run.NamespaceID, run.PipelineID, run.ID, &taskStatusMap)
	if err != nil {
		log.Error().Err(err).Msg("could not update run status")
		return
	}
}

// monitorRunStatus takes into account all task run statuses that are currently being run and then determines
// the final run status based on all finished task run statuses. It will block until all task runs have finished.
func (api *API) monitorRunStatus(namespaceID, pipelineID string, runID int64, statusMap *syncmap.Syncmap[string, models.ContainerState]) error {
	run, err := api.storage.GetRun(storage.GetRunRequest{
		NamespaceID: namespaceID,
		PipelineID:  pipelineID,
		ID:          runID,
	})
	if err != nil {
		return err
	}

	run.State = models.RunRunning

	err = api.storage.UpdateRun(storage.UpdateRunRequest{Run: run})
	if err != nil {
		return err
	}

	failures := 0
	finished := 0
	cancelled := 0
	for {
		time.Sleep(time.Second * 3)
		statusList := []models.ContainerState{}
		for _, key := range statusMap.Keys() {
			status, exists := statusMap.Get(key)
			if !exists {
				continue
			}

			statusList = append(statusList, status)
		}

		failures = 0
		finished = 0
		cancelled = 0
		total := len(statusList)

		for _, status := range statusList {
			switch status {
			case models.ContainerStateFailed:
				failures++
				finished++
			case models.ContainerStateSuccess, models.ContainerStateSkipped:
				finished++
			case models.ContainerStateCancelled:
				finished++
				cancelled++
			}
		}

		if finished != total {
			continue
		}

		break
	}

	run, err = api.storage.GetRun(storage.GetRunRequest{
		NamespaceID: namespaceID,
		PipelineID:  pipelineID,
		ID:          runID,
	})
	if err != nil {
		return err
	}

	switch {
	case cancelled > 0:
		run.SetCancelled("One or more task runs were cancelled during execution.")
	case failures > 0:
		run.SetFailed(models.RunFailureKindAbnormalExit, "One or more task runs failed during execution.")
	default:
		run.SetSucceeded()
	}

	err = api.storage.UpdateRun(storage.UpdateRunRequest{Run: run})
	if err != nil {
		return err
	}

	api.events.Publish(models.NewEventCompletedRun(*run))

	log.Info().Int64("id", run.ID).Str("pipeline", run.PipelineID).
		Str("result", string(run.State)).Msg("finished run")

	return nil
}

// cancelRun cancels all task runs related to a run by calling the scheduler's StopContainer function on each one.
// It then waits until the goroutine monitoring run health gets to the correct state.
// This causes the function to block for a bit, while it waits for the correct run status.
func (api *API) cancelRun(run *models.Run, description string, force bool) error {
	taskRuns, err := api.storage.GetAllTaskRuns(storage.GetAllTaskRunsRequest{
		NamespaceID: run.NamespaceID,
		PipelineID:  run.PipelineID,
		RunID:       run.ID,
	})
	if err != nil {
		return err
	}

	// Because of how state updates work we need to wait for the run to be settled by
	// the goroutine that controls this before we update the description.
	for {
		for _, taskrun := range taskRuns {
			if taskrun.State != models.ContainerStateRunning {
				continue
			}

			err := api.cancelTaskRun(taskrun, force)
			if err != nil {
				if errors.Is(err, scheduler.ErrNoSuchContainer) {
					taskrun.SetFinishedAbnormal(models.ContainerStateFailed, models.TaskRunFailure{
						Kind:        models.TaskRunFailureKindOrphaned,
						Description: "Scheduler could not find task run when queried.",
					}, 1)

					err = api.storage.UpdateRun(storage.UpdateRunRequest{Run: run})
					if err != nil {
						return err
					}
				}

				log.Error().Err(err).
					Str("id", taskrun.ID).
					Str("pipeline", taskrun.PipelineID).
					Int64("run", taskrun.RunID).
					Msg("could not cancel task run")
			}
		}

		run, err := api.storage.GetRun(storage.GetRunRequest{
			NamespaceID: run.NamespaceID,
			PipelineID:  run.PipelineID,
			ID:          run.ID,
		})
		if err != nil {
			return err
		}

		if run.State == models.RunRunning {
			time.Sleep(time.Second * 5)
			continue
		}

		if run.State == models.RunFailed ||
			run.State == models.RunSuccess {
			return nil
		}

		if run.State == models.RunCancelled {
			run.Failure.Description = description
			err = api.storage.UpdateRun(storage.UpdateRunRequest{Run: run})
			if err != nil {
				return err
			}
			return nil
		}

		time.Sleep(time.Second * 5)
	}
}

func (api *API) cancelAllRuns(namespaceID, pipelineID, description string, force bool) ([]int64, error) {
	type runkey struct {
		namespace string
		pipeline  string
		run       int64
	}

	// Collect all events.
	events := api.events.GetAll(false)
	inProgressRunMap := map[runkey]struct{}{}

	// Search events for any orphan runs.
	for event := range events {
		switch evt := event.(type) {
		case *models.EventStartedRun:
			key := runkey{
				namespace: evt.NamespaceID,
				pipeline:  evt.PipelineID,
				run:       evt.RunID,
			}

			if key.namespace != namespaceID || key.pipeline != pipelineID {
				continue
			}

			_, exists := inProgressRunMap[key]

			if !exists {
				inProgressRunMap[key] = struct{}{}
			}

		case *models.EventCompletedRun:
			_, exists := inProgressRunMap[runkey{
				namespace: evt.NamespaceID,
				pipeline:  evt.PipelineID,
				run:       evt.RunID,
			}]

			if exists {
				delete(inProgressRunMap, runkey{
					namespace: evt.NamespaceID,
					pipeline:  evt.PipelineID,
					run:       evt.RunID,
				})
			}
		}
	}

	// Retrieve actual runs
	inProgressRuns := []*models.Run{}

	for inProgressRun := range inProgressRunMap {
		run, err := api.storage.GetRun(storage.GetRunRequest{
			NamespaceID: inProgressRun.namespace,
			PipelineID:  inProgressRun.pipeline,
			ID:          inProgressRun.run,
		})
		if err != nil {
			log.Error().Err(err).Str("namespace", inProgressRun.namespace).Str("pipeline", inProgressRun.pipeline).
				Int64("run", inProgressRun.run).Msg("could not retrieve run from database")
			continue
		}

		inProgressRuns = append(inProgressRuns, run)
	}

	var wg sync.WaitGroup
	cancelledRunList := []int64{}

	for _, run := range inProgressRuns {
		run := run

		// If run is in a finished state just skip over it.
		if run.IsComplete() {
			continue
		}

		// Launch to routines to handle all in-progress run cancellations. We call cancelrun and then wait until the
		// state has reached a "finished" state.
		cancelledRunList = append(cancelledRunList, run.ID)
		wg.Add(1)
		go func(run *models.Run) {
			defer wg.Done()
			_ = api.cancelRun(run, description, force)
			for {
				run, err := api.storage.GetRun(storage.GetRunRequest{
					NamespaceID: run.NamespaceID,
					PipelineID:  run.PipelineID,
					ID:          run.ID,
				})
				if err != nil {
					time.Sleep(time.Second * 3)
					continue
				}

				if run.IsComplete() {
					return
				}

				time.Sleep(time.Second * 3)
			}
		}(run)
	}

	wg.Wait()
	return cancelledRunList, nil
}

func sliceToSet(elements []string) map[string]struct{} {
	elementMap := make(map[string]struct{})
	for _, s := range elements {
		elementMap[s] = struct{}{}
	}
	return elementMap
}
