package api

import (
	"fmt"

	"github.com/clintjedwards/gofer/internal/config"
	"github.com/clintjedwards/gofer/internal/models"
	"github.com/rs/zerolog/log"
)

func (api *API) installNotifiersFromConfig() {
	for _, notifier := range api.config.Notifiers.RegisteredNotifiers {
		_, exists := api.notifiers[notifier.Kind]
		if exists {
			continue
		}
		api.registerNotifier(notifier)
	}
}

func (api *API) registerNotifier(notifier config.Notifier) {
	api.notifiers[notifier.Kind] = &models.Notifier{
		Kind:          notifier.Kind,
		Image:         notifier.Image,
		Documentation: fmt.Sprintf("https://clintjedwards.com/gofer/docs/notifiers/%s/overview", notifier.Kind),
		RegistryAuth: models.RegistryAuth{
			User: notifier.User,
			Pass: notifier.Pass,
		},
		EnvVars: notifier.EnvVars,
	}

	log.Info().Str("kind", notifier.Kind).Msg("registered notifier")
}
