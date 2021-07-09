package bolt

// AddDockerRegistryAuth(r AddDockerRegistryAuthRequest)
// RemoveDockerRegistryAuth(r RemoveDockerRegistryAuthRequest)
// ListDockerRegistryAuths(r ListDockerRegistryAuthsRequest)

import (
	"errors"

	"github.com/asdine/storm/v3"
	"github.com/clintjedwards/gofer/internal/models"
	"github.com/clintjedwards/gofer/internal/storage"
)

func (db *DB) GetAllDockerRegistryAuths(r storage.GetAllDockerRegistryAuthsRequest) ([]*models.DockerRegistryAuth, error) {
	dockerregistryauth := []*models.DockerRegistryAuth{}
	err := db.All(&dockerregistryauth)
	if err != nil {
		return nil, err
	}

	return dockerregistryauth, nil
}

// GetDockerRegistryAuth returns a single dockerregistryauth by id.
func (db *DB) GetDockerRegistryAuth(r storage.GetDockerRegistryAuthRequest) (*models.DockerRegistryAuth, error) {
	var dockerregistryauth models.DockerRegistryAuth
	err := db.One("Registry", r.Registry, &dockerregistryauth)
	if err != nil {
		if errors.Is(err, storm.ErrNotFound) {
			return nil, storage.ErrEntityNotFound
		}

		return nil, err
	}

	return &dockerregistryauth, nil
}

func (db *DB) AddDockerRegistryAuth(r storage.AddDockerRegistryAuthRequest) error {
	tx, err := db.Begin(true)
	if err != nil {
		return err
	}
	defer tx.Rollback() // nolint: errcheck

	var dockerregistryauth models.DockerRegistryAuth
	err = tx.One("Registry", r.DockerRegistryAuth.Registry, &dockerregistryauth)
	if errors.Is(err, storm.ErrNotFound) {
		err = tx.Save(r.DockerRegistryAuth)
		if err != nil {
			if errors.Is(err, storm.ErrNoID) {
				return storage.ErrPreconditionFailure
			}

			return err
		}

		return tx.Commit()
	}

	if err == nil {
		return storage.ErrEntityExists
	}

	return err
}

func (db *DB) RemoveDockerRegistryAuth(r storage.RemoveDockerRegistryAuthRequest) error {
	err := db.DeleteStruct(&models.DockerRegistryAuth{Registry: r.Registry})
	if err != nil {
		if errors.Is(err, storm.ErrNotFound) {
			return storage.ErrEntityNotFound
		}

		if errors.Is(err, storm.ErrNoID) {
			return storage.ErrPreconditionFailure
		}

		return err
	}

	return nil
}
