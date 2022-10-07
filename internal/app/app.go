// Package app is the setup package for all things API related. It calls properly initializes all other
// required API functions and starts the main API service.
package app

import (
	"fmt"

	"github.com/clintjedwards/gofer/internal/api"
	"github.com/clintjedwards/gofer/internal/config"
	objectstore "github.com/clintjedwards/gofer/internal/objectStore"
	sqliteos "github.com/clintjedwards/gofer/internal/objectStore/sqlite"
	"github.com/clintjedwards/gofer/internal/scheduler"
	"github.com/clintjedwards/gofer/internal/scheduler/docker"
	"github.com/clintjedwards/gofer/internal/secretStore"
	sqlitesecret "github.com/clintjedwards/gofer/internal/secretStore/sqlite"
	"github.com/clintjedwards/gofer/internal/storage"
	"github.com/rs/zerolog/log"
)

// StartServices initializes all required services.
func StartServices(config *config.API) {
	if config.DevMode {
		log.Warn().Msg("server in development mode; not for use in production")
	}

	newStorage, err := initStorage(config.Server)
	if err != nil {
		log.Fatal().Err(err).Msg("could not init storage")
	}

	log.Info().Str("path", config.Server.StoragePath).Int("max_results_limit", config.Server.StorageResultsLimit).
		Msg("storage initialized")

	newScheduler, err := initScheduler(config.Scheduler)
	if err != nil {
		log.Fatal().Err(err).Msg("could not init scheduler")
	}

	log.Info().Str("engine", config.Scheduler.Engine).Msg("scheduler engine initialized")

	newObjectStore, err := initObjectStore(config.ObjectStore)
	if err != nil {
		log.Fatal().Err(err).Msg("could not init objectStore")
	}

	log.Info().Str("engine", config.ObjectStore.Engine).Msg("object store engine initialized")

	newSecretStore, err := initSecretStore(config.SecretStore)
	if err != nil {
		log.Fatal().Err(err).Msg("could not init secretStore")
	}

	log.Info().Str("engine", config.SecretStore.Engine).Msg("secret store engine initialized")

	newAPI, err := api.NewAPI(config, newStorage, newScheduler, newObjectStore, newSecretStore)
	if err != nil {
		log.Fatal().Err(err).Msg("could not init api")
	}

	if config.ExternalEventsAPI.Enable {
		go api.StartExternalEventsService(config, newAPI)
	}
	newAPI.StartAPIService()
}

// initStorage creates a storage object with the appropriate engine
func initStorage(config *config.Server) (storage.DB, error) {
	return storage.New(config.StoragePath, config.StorageResultsLimit)
}

func initObjectStore(config *config.ObjectStore) (objectstore.Engine, error) {
	switch objectstore.EngineType(config.Engine) {
	case objectstore.EngineSqlite:
		engine, err := sqliteos.New(config.Sqlite.Path)
		if err != nil {
			return nil, err
		}

		return &engine, err
	default:
		return nil, fmt.Errorf("object store backend %q not implemented", config.Engine)
	}
}

func initSecretStore(config *config.SecretStore) (secretStore.Engine, error) {
	switch secretStore.EngineType(config.Engine) {
	case secretStore.EngineSqlite:
		engine, err := sqlitesecret.New(config.Sqlite.Path, config.Sqlite.EncryptionKey)
		if err != nil {
			return nil, err
		}

		return &engine, err
	default:
		return nil, fmt.Errorf("secret backend %q not implemented", config.Engine)
	}
}

func initScheduler(config *config.Scheduler) (scheduler.Engine, error) {
	switch scheduler.EngineType(config.Engine) {
	case scheduler.EngineDocker:
		engine, err := docker.New(config.Docker.Prune, config.Docker.PruneInterval)
		if err != nil {
			return nil, err
		}

		return &engine, err
	default:
		return nil, fmt.Errorf("scheduler backend %q not implemented", config.Engine)
	}
}
