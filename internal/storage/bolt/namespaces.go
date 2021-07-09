package bolt

import (
	"errors"

	"github.com/asdine/storm/v3"
	"github.com/clintjedwards/gofer/internal/models"
	"github.com/clintjedwards/gofer/internal/storage"
)

func (db *DB) GetAllNamespaces(r storage.GetAllNamespacesRequest) ([]*models.Namespace, error) {
	if r.Limit == 0 || r.Limit > db.maxResultsLimit {
		r.Limit = db.maxResultsLimit
	}

	namespaces := []*models.Namespace{}
	err := db.All(&namespaces, storm.Limit(r.Limit), storm.Skip(r.Offset))
	if err != nil {
		return nil, err
	}

	return namespaces, nil
}

// GetNamespace returns a single namespace by id.
func (db *DB) GetNamespace(r storage.GetNamespaceRequest) (*models.Namespace, error) {
	var namespace models.Namespace
	err := db.One("ID", r.ID, &namespace)
	if err != nil {
		if errors.Is(err, storm.ErrNotFound) {
			return nil, storage.ErrEntityNotFound
		}

		return nil, err
	}

	return &namespace, nil
}

func (db *DB) AddNamespace(r storage.AddNamespaceRequest) error {
	tx, err := db.Begin(true)
	if err != nil {
		return err
	}
	defer tx.Rollback() // nolint: errcheck

	var namespace models.Namespace
	err = tx.One("ID", r.Namespace.ID, &namespace)
	if errors.Is(err, storm.ErrNotFound) {
		err = tx.Save(r.Namespace)
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

func (db *DB) UpdateNamespace(r storage.UpdateNamespaceRequest) error {
	err := db.Update(r.Namespace)
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
