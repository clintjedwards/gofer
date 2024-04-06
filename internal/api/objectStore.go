//go:build ignore

package api

import (
	"strings"

	"github.com/clintjedwards/gofer/internal/models"
	"github.com/clintjedwards/gofer/internal/storage"
	"github.com/rs/zerolog/log"
)

// addPipelineObject adds an object to the pipeline specific object registry.
// If this registry is at the limit it removes the least recently added pipeline object and
// puts the new item on top.
func (api *APIContext) addPipelineObject(namespace, pipeline string, key string, content []byte, force bool) (string, error) {
	objectKeys, err := api.db.ListObjectStorePipelineKeys(api.db, namespace, pipeline)
	if err != nil {
		return "", err
	}

	newObjectKey := models.NewObjectStoreKey(key)

	err = api.db.InsertObjectStorePipelineKey(api.db, &storage.ObjectStorePipelineKey{
		Namespace: namespace,
		Pipeline:  pipeline,
		Key:       newObjectKey.Key,
		Created:   newObjectKey.Created,
	})
	if err != nil {
		return "", err
	}

	err = api.objectStore.PutObject(pipelineObjectKey(namespace, pipeline, key), content, force)
	if err != nil {
		return "", err
	}

	isExistingKey := false
	for _, object := range objectKeys {
		if strings.EqualFold(key, object.Key) {
			isExistingKey = true
		}
	}

	evictedObjectKey := ""
	if len(objectKeys) >= api.config.ObjectStore.PipelineObjectLimit && !isExistingKey {
		err := api.objectStore.DeleteObject(pipelineObjectKey(namespace, pipeline, objectKeys[0].Key))
		if err != nil {
			log.Error().Err(err).Msg("could not delete pipeline object")
		}
		evictedObjectKey = objectKeys[0].Key
	}

	return evictedObjectKey, nil
}
