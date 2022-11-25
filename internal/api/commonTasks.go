package api

import (
	"github.com/clintjedwards/gofer/internal/models"
	"github.com/rs/zerolog/log"
)

func (api *API) restoreRegisteredCommonTasks() error {
	registeredCommonTasks, err := api.db.ListCommonTaskRegistrations(api.db, 0, 0)
	if err != nil {
		return err
	}

	for _, commonTaskRaw := range registeredCommonTasks {
		var commonTask models.CommonTaskRegistration
		commonTask.FromStorage(&commonTaskRaw)
		api.commonTasks.Set(commonTask.Name, &commonTask)

		log.Info().Str("name", commonTask.Name).Msg("restored common task registration")
	}

	return nil
}
