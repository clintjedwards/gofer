package api

import (
	"fmt"

	"github.com/clintjedwards/gofer/internal/models"
)

func (api *API) registerNotifiers() {
	for _, notifier := range api.config.Notifiers.RegisteredNotifiers {
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
	}
}
