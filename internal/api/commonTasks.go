package api

import (
	"github.com/clintjedwards/gofer/models"
	"github.com/rs/zerolog/log"
)

func (api *API) restoreRegisteredCommonTasks() error {
	registeredCommonTasks, err := api.db.ListCommonTaskRegistrations(0, 0)
	if err != nil {
		return err
	}

	for _, commonTask := range registeredCommonTasks {
		api.commonTasks.Set(commonTask.Name, &models.CommonTaskRegistration{
			Name:          commonTask.Name,
			Image:         commonTask.Image,
			RegistryAuth:  commonTask.RegistryAuth,
			Variables:     commonTask.Variables,
			Documentation: commonTask.Documentation,
		})

		log.Info().Str("name", commonTask.Name).Msg("restored common task registration")
	}

	return nil
}
