package api

import (
	"errors"
	"fmt"
	"strconv"
	"strings"
	"sync"
	"time"

	"github.com/clintjedwards/gofer/internal/models"
	"github.com/clintjedwards/gofer/internal/objectStore"
	"github.com/clintjedwards/gofer/internal/scheduler"
	"github.com/clintjedwards/gofer/internal/secretStore"
	"github.com/clintjedwards/gofer/internal/storage"
	"github.com/rs/zerolog/log"
)

const (
	// GOFEREOF is a special string marker we include at the end of log files.
	// It denotes that no further logs will be written. This is to provide the functionality for downstream
	// applications to follow log files and not also have to monitor the container for state to know when
	// logs will no longer be printed.
	GOFEREOF string = "GOFER_EOF"
)

type InterpolationKind string

const (
	InterpolationKindUnknown        InterpolationKind = "UNKNOWN"
	InterpolationKindPipelineSecret InterpolationKind = "PIPELINE_SECRET"
	InterpolationKindGlobalSecret   InterpolationKind = "GLOBAL_SECRET"
	InterpolationKindPipelineObject InterpolationKind = "PIPELINE_OBJECT"
	InterpolationKindRunObject      InterpolationKind = "RUN_OBJECT"
)

// Checks a string for the existence of a interpolation format. ex: "secret{{ example }}".
// If an interpolation was found we returns some, if not we return none.
//
// Currently the supported interpolation syntaxes are:
//   - `pipeline_secret{{ example }}` for inserting from the pipeline secret store.
//   - `global_secret{{ example }}` for inserting from the global secret store.
//   - `pipeline_object{{ example }}` for inserting from the pipeline object store.
//   - `run_object{{ example }}` for inserting from the run object store.
func parseInterpolationSyntax(kind InterpolationKind, input string) (string, error) {
	variable := strings.TrimSpace(input)
	interpolationPrefix := fmt.Sprintf("%s{{", strings.ToLower(string(kind)))
	interpolationSuffix := "}}"
	if strings.HasPrefix(variable, interpolationPrefix) && strings.HasSuffix(variable, interpolationSuffix) {
		variable = strings.TrimPrefix(variable, interpolationPrefix)
		variable = strings.TrimSuffix(variable, interpolationSuffix)
		return strings.TrimSpace(variable), nil
	}

	return "", fmt.Errorf("variable doesn't match interpolation prefix/suffix")
}

func systemInjectedVars(run *models.Run, task models.Task, injectToken bool) map[string]*models.Variable {
	vars := map[string]*models.Variable{
		"GOFER_PIPELINE_ID": {
			Key:    "GOFER_PIPELINE_ID",
			Value:  run.Pipeline,
			Source: models.VariableSourceSystem,
		},
		"GOFER_RUN_ID": {
			Key:    "GOFER_RUN_ID",
			Value:  strconv.FormatInt(run.ID, 10),
			Source: models.VariableSourceSystem,
		},
		"GOFER_TASK_ID": {
			Key:    "GOFER_TASK_ID",
			Value:  task.GetID(),
			Source: models.VariableSourceSystem,
		},
		"GOFER_TASK_IMAGE": {
			Key:    "GOFER_TASK_IMAGE",
			Value:  task.GetImage(),
			Source: models.VariableSourceSystem,
		},
	}

	if injectToken {
		vars["GOFER_API_TOKEN"] = &models.Variable{
			Key:    "GOFER_API_TOKEN",
			Value:  fmt.Sprintf("secret{{%s}}", fmt.Sprintf("gofer_api_token_%d", run.ID)),
			Source: models.VariableSourceSystem,
		}
	}

	return vars
}

// We need to combine the environment variables we get from multiple sources in order to pass them
// finally to the task run. The order in which they are passed is very important as they can and should
// overwrite each other, even though the intention of prefixing the environment variables is to prevent
// the chance of overwriting. The order in which they are passed into the extend function
// determines the priority in reverse order. Last in the stack will overwrite any conflicts from the others.
//
// There are many places a task_run could potentially get env vars from. From the outer most layer to the inner most:
// 1) The user sets variables in their pipeline configuration for each task.
// 2) At the time of run inception, either the extension or the user themselves have the ability to inject extra env vars.
// 3) Right before the task run starts, Gofer itself might inject variables into the task run.
//
// The order in which the env vars are stacked are in reverse order to the above, due to that order being the best
// for giving the user the most control over what the pipeline does:
// 1) We first pass in the Gofer system specific envvars as these are the most replaceable on the totem pole.
// 2) We pass in the task specific envvars defined by the user in the pipeline config.
// 3) Lastly we pass in the run specific defined envvars. These are usually provided by either a extension
// or the user when they attempt to start a new run manually. Since these are the most likely to be
// edited adhoc they are treated as the most important.
func combineVariables(run *models.Run, task models.Task) []models.Variable {
	systemInjectedVars := systemInjectedVars(run, task, task.GetInjectAPIToken())

	taskVars := map[string]*models.Variable{}
	for _, variable := range task.GetVariables() {
		variable := variable
		taskVars[strings.ToUpper(variable.Key)] = &variable
	}

	runVars := map[string]*models.Variable{}
	for _, variable := range run.Variables {
		variable := variable
		runVars[variable.Key] = &variable
	}

	taskRunVars := mergeMaps(
		systemInjectedVars, // Gofer provided env vars first.
		taskVars,           // Then we include vars that come from the pipeline config.
		runVars,            // Then finally vars that come from the user or the extension.
	)

	variables := []models.Variable{}
	for key, value := range taskRunVars {
		value := value
		if key == "" {
			// It is possible for the user to enter an empty key, but that would be an error when
			// attempting to pass it to the docker container.
			delete(taskRunVars, key)
			continue
		}

		variables = append(variables, *value)
	}

	return variables
}

// mergeVariableMaps combines many string maps in a "last one in wins" format. Meaning that in case of key collision
// the last map to be added will overwrite the value of the previous key.
func mergeMaps[ValueType string | *models.Variable](maps ...map[string]ValueType) map[string]ValueType {
	newMap := map[string]ValueType{}

	for _, extraMap := range maps {
		for key, value := range extraMap {
			value := value
			newMap[key] = value
		}
	}

	return newMap
}

// Takes a map (usually passed in from a source that does not care about ownership/privacy) and converts it into a slice
// of Variable objects.
func convertVarsToSlice(vars map[string]string, source models.VariableSource) []models.Variable {
	variables := []models.Variable{}

	for key, value := range vars {
		key := key
		value := value
		variables = append(variables, models.Variable{
			Key:    key,
			Value:  value,
			Source: source,
		})
	}

	return variables
}

// Takes a slice of variable objects and converts it into a map. This is usually passed to a part of the program that does
// not care about variables.
func convertVarsToMap(vars []models.Variable) map[string]string {
	variables := map[string]string{}

	for _, variable := range vars {
		variable := variable
		variables[strings.ToUpper(variable.Key)] = variable.Value
	}

	return variables
}

// Takes in a map of mixed plaintext and raw secret/store strings and populates it with
// the fetched strings for each type.
func (api *API) interpolateVars(namespace, pipeline string, run *int64, variables []models.Variable) ([]models.Variable, error) {
	varList := []models.Variable{}

	for _, variable := range variables {
		key, err := parseInterpolationSyntax(InterpolationKindPipelineSecret, variable.Value)
		if err == nil {
			variable := variable
			value, err := api.secretStore.GetSecret(pipelineSecretKey(namespace, pipeline, key))
			if err != nil {
				if errors.Is(err, secretStore.ErrEntityNotFound) {
					return nil, fmt.Errorf("could not find pipeline secret %q", key)
				}
				return nil, err
			}

			variable.Value = value
			varList = append(varList, variable)
			continue
		}

		key, err = parseInterpolationSyntax(InterpolationKindGlobalSecret, variable.Value)
		if err == nil {
			keyMetadataRaw, err := api.db.GetSecretStoreGlobalKey(api.db, key)
			if err != nil {
				if errors.Is(err, storage.ErrEntityNotFound) {
					return nil, fmt.Errorf("could not find global secret %q", key)
				}
				return nil, err
			}

			keyMetadata := models.SecretStoreKey{}
			keyMetadata.FromGlobalSecretKeyStorage(&keyMetadataRaw)
			if !keyMetadata.IsAllowedNamespace(namespace) {
				return nil, fmt.Errorf("global secret %q cannot be used in this current namespace. Valid namespaces: %v",
					key, keyMetadata.Namespaces)
			}

			variable := variable
			value, err := api.secretStore.GetSecret(globalSecretKey(key))
			if err != nil {
				if errors.Is(err, secretStore.ErrEntityNotFound) {
					return nil, fmt.Errorf("could not find global secret %q", key)
				}
				return nil, err
			}

			variable.Value = value
			varList = append(varList, variable)
			continue
		}

		key, err = parseInterpolationSyntax(InterpolationKindPipelineObject, variable.Value)
		if err == nil {
			variable := variable
			value, err := api.objectStore.GetObject(pipelineObjectKey(namespace, pipeline, key))
			if err != nil {
				if errors.Is(err, objectStore.ErrEntityNotFound) {
					return nil, fmt.Errorf("could not find pipeline object %q", key)
				}
				return nil, err
			}

			variable.Value = string(value)
			varList = append(varList, variable)
			continue
		}

		if run != nil {
			key, err = parseInterpolationSyntax(InterpolationKindRunObject, variable.Value)
			if err == nil {
				variable := variable
				value, err := api.objectStore.GetObject(runObjectKey(namespace, pipeline, *run, key))
				if err != nil {
					if errors.Is(err, objectStore.ErrEntityNotFound) {
						return nil, fmt.Errorf("could not find run object %q", key)
					}
					return nil, err
				}

				variable.Value = string(value)
				varList = append(varList, variable)
				continue
			}
		}

		varList = append(varList, variable)
	}

	return varList, nil
}

// cancelRun cancels all task runs related to a run by calling the scheduler's StopContainer function on each one.
// It then waits until the goroutine monitoring run health gets to the correct state.
// This causes the function to block for a bit, while it waits for the correct run status.
func (api *API) cancelRun(run *models.Run, description string, force bool) error {
	taskRunsRaw, err := api.db.ListPipelineTaskRuns(api.db, 0, 0, run.Namespace, run.Pipeline, run.ID)
	if err != nil {
		return err
	}

	// Because of how state updates work we need to wait for the run to be settled by
	// the goroutine that controls this before we update the description.
	for {
		for _, taskrunRaw := range taskRunsRaw {
			var taskrun models.TaskRun
			taskrun.FromStorage(&taskrunRaw)

			if taskrun.State != models.TaskRunStateRunning {
				continue
			}

			err := api.cancelTaskRun(&taskrun, force)
			if err != nil {
				if errors.Is(err, scheduler.ErrNoSuchContainer) {

					statusReason := models.TaskRunStatusReason{
						Reason:      models.TaskRunStatusReasonKindOrphaned,
						Description: "Scheduler could not find task run when queried.",
					}

					err = api.db.UpdatePipelineTaskRun(api.db, taskrun.Namespace, taskrun.Pipeline, taskrun.Run,
						taskrun.ID, storage.UpdatablePipelineTaskRunFields{
							Status:       ptr(string(models.TaskRunStatusFailed)),
							State:        ptr(string(models.TaskRunStateComplete)),
							Ended:        ptr(time.Now().UnixMilli()),
							StatusReason: ptr(string(statusReason.ToJSON())),
						})
					if err != nil {
						return err
					}

					go api.events.Publish(models.EventCompletedTaskRun{
						NamespaceID: taskrun.Namespace,
						PipelineID:  taskrun.Pipeline,
						RunID:       taskrun.Run,
						TaskRunID:   taskrun.ID,
						Status:      models.TaskRunStatusFailed,
					})

					return nil
				}
			}
		}

		runRaw, err := api.db.GetPipelineRun(api.db, run.Namespace, run.Pipeline, run.ID)
		if err != nil {
			return err
		}

		var run models.Run
		run.FromStorage(&runRaw)

		if run.State != models.RunStateComplete {
			time.Sleep(time.Second * 1)
			continue
		}

		if run.Status == models.RunStatusFailed ||
			run.Status == models.RunStatusSuccessful {
			return nil
		}

		if run.Status == models.RunStatusCancelled {
			statusReason := models.RunStatusReason{
				Reason:      models.RunStatusReasonKindUserCancelled,
				Description: description,
			}

			err = api.db.UpdatePipelineRun(api.db, run.Namespace, run.Pipeline, run.ID, storage.UpdatablePipelineRunFields{
				StatusReason: ptr(string(statusReason.ToJSON())),
			})
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
		switch evt := event.Details.(type) {
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
		runRaw, err := api.db.GetPipelineRun(api.db, inProgressRun.namespace, inProgressRun.pipeline, inProgressRun.run)
		if err != nil {
			log.Error().Err(err).Str("namespace", inProgressRun.namespace).Str("pipeline", inProgressRun.pipeline).
				Int64("run", inProgressRun.run).Msg("could not retrieve run from database")
			continue
		}

		var run models.Run
		run.FromStorage(&runRaw)

		inProgressRuns = append(inProgressRuns, &run)
	}

	var wg sync.WaitGroup
	cancelledRunList := []int64{}

	for _, run := range inProgressRuns {
		run := run

		// If run is in a finished state just skip over it.
		if run.State == models.RunStateComplete {
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
				runRaw, err := api.db.GetPipelineRun(api.db, run.Namespace, run.Pipeline, run.ID)
				if err != nil {
					time.Sleep(time.Second * 3)
					continue
				}

				var run models.Run
				run.FromStorage(&runRaw)

				if run.State == models.RunStateComplete {
					return
				}

				time.Sleep(time.Second * 3)
			}
		}(run)
	}

	wg.Wait()
	return cancelledRunList, nil
}
