// Package app is the setup package for all things API related. It calls properly initializes all other
// required API functions and starts the main API service.
package app

import (
	"fmt"

	"github.com/clintjedwards/gofer/internal/api"
	"github.com/clintjedwards/gofer/internal/config"
	objectstore "github.com/clintjedwards/gofer/internal/objectStore"
	boltos "github.com/clintjedwards/gofer/internal/objectStore/bolt"
	"github.com/clintjedwards/gofer/internal/scheduler"
	"github.com/clintjedwards/gofer/internal/scheduler/docker"
	"github.com/clintjedwards/gofer/internal/storage"
	"github.com/clintjedwards/gofer/internal/storage/bolt"
	"github.com/rs/zerolog/log"
)

// StartServices initializes all required services.
func StartServices(config *config.API) {
	if config.Server.DevMode {
		log.Warn().Msg("server in development mode; not for use in production")
	}

	newStorage, err := initStorage(config.Database)
	if err != nil {
		log.Fatal().Err(err).Msg("could not init storage")
	}

	log.Info().Str("engine", config.Database.Engine).Msg("storage engine initialized")

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

	newAPI, err := api.NewAPI(config, newStorage, newScheduler, newObjectStore)
	if err != nil {
		log.Fatal().Err(err).Msg("could not init api")
	}

	if config.ExternalEventsAPI.Enable {
		go api.StartEventsService(config, newAPI)
	}
	newAPI.StartAPIService()
}

// initStorage creates a storage object with the appropriate engine
func initStorage(config *config.Database) (storage.Engine, error) {
	switch storage.EngineType(config.Engine) {
	case storage.StorageEngineBoltDB:
		boltStorageEngine, err := bolt.New(config.BoltDB.Path, config.MaxResultsLimit)
		if err != nil {
			return nil, err
		}

		return &boltStorageEngine, nil
	default:
		return nil, fmt.Errorf("storage backend %q not implemented", config.Engine)
	}
}

func initObjectStore(config *config.ObjectStore) (objectstore.Engine, error) {
	switch objectstore.EngineType(config.Engine) {
	case objectstore.EngineBolt:
		engine, err := boltos.New(config.BoltDB.Path)
		if err != nil {
			return nil, err
		}

		return &engine, err
	default:
		return nil, fmt.Errorf("scheduler backend %q not implemented", config.Engine)
	}
}

func initScheduler(config *config.Scheduler) (scheduler.Engine, error) {
	switch scheduler.EngineType(config.Engine) {
	case scheduler.EngineDocker:
		engine, err := docker.New(config.Docker.Prune, config.Docker.PruneInterval, config.Docker.SecretsPath)
		if err != nil {
			return nil, err
		}

		return &engine, err
	default:
		return nil, fmt.Errorf("scheduler backend %q not implemented", config.Engine)
	}
}
