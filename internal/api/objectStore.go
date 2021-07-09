package api

import (
	"fmt"

	"github.com/clintjedwards/gofer/internal/storage"
)

const (
	ObjectPipelineKeyFmt = "%s_%s_%s"    // namespaceid_pipelineid_key
	ObjectRunKeyFmt      = "%s_%s_%d_%s" //  namespaceid_pipelineid_runid_key
)

func pipelineObjectKey(namespace, pipeline, key string) string {
	return fmt.Sprintf(ObjectPipelineKeyFmt, namespace, pipeline, key)
}

func runObjectKey(namespace, pipeline, key string, runID int64) string {
	return fmt.Sprintf(ObjectRunKeyFmt, namespace, pipeline, runID, key)
}

// addPipelineObject adds an object to the pipeline specific object registry.
// If this registry is at the limit it removes the least recently added pipeline object and
// puts the new item on top.
func (api *API) addPipelineObject(namespace, pipeline, key string, content []byte, force bool) (string, error) {
	err := api.objectStore.PutObject(pipelineObjectKey(namespace, pipeline, key), content, force)
	if err != nil {
		return "", err
	}

	currentPipeline, err := api.storage.GetPipeline(storage.GetPipelineRequest{
		NamespaceID: namespace,
		ID:          pipeline,
	})
	if err != nil {
		_ = api.objectStore.DeleteObject(pipelineObjectKey(namespace, pipeline, key))
		return "", err
	}

	isCurrentKey := false
	for _, object := range currentPipeline.Objects {
		if key == object {
			isCurrentKey = true
		}
	}

	evictedObject := ""
	if len(currentPipeline.Objects) >= api.config.ObjectStore.PipelineObjectLimit && !isCurrentKey {
		_ = api.objectStore.DeleteObject(pipelineObjectKey(namespace, pipeline, currentPipeline.Objects[len(currentPipeline.Objects)-1]))
		evictedObject = currentPipeline.Objects[len(currentPipeline.Objects)-1]
		currentPipeline.Objects[len(currentPipeline.Objects)-1] = "" // zero value to prevent memory leaks
		currentPipeline.Objects = currentPipeline.Objects[:len(currentPipeline.Objects)-1]
	}

	if !isCurrentKey {
		currentPipeline.Objects = append([]string{key}, currentPipeline.Objects...)
	}

	err = api.storage.UpdatePipeline(storage.UpdatePipelineRequest{Pipeline: currentPipeline})
	if err != nil {
		_ = api.objectStore.DeleteObject(pipelineObjectKey(namespace, pipeline, key))
		return "", err
	}

	return evictedObject, nil
}

// addRunObject simply adds an object for a specific pipeline run. Run objects only last over a set number of runs.
func (api *API) addRunObject(namespace, pipeline, key string, runID int64, content []byte, force bool) error {
	err := api.objectStore.PutObject(runObjectKey(namespace, pipeline, key, runID), content, force)
	if err != nil {
		return err
	}

	currentRun, err := api.storage.GetRun(storage.GetRunRequest{NamespaceID: namespace, PipelineID: pipeline, ID: runID})
	if err != nil {
		_ = api.objectStore.DeleteObject(runObjectKey(namespace, pipeline, key, runID))
		return err
	}

	currentRun.Objects = append(currentRun.Objects, key)

	err = api.storage.UpdateRun(storage.UpdateRunRequest{Run: currentRun})
	if err != nil {
		_ = api.objectStore.DeleteObject(runObjectKey(namespace, pipeline, key, runID))
		return err
	}

	return nil
}
