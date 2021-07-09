package bolt

import (
	"errors"

	"github.com/asdine/storm/v3"
	"github.com/clintjedwards/gofer/internal/models"
	"github.com/clintjedwards/gofer/internal/storage"
)

// GetAllPipelines returns all pipelines with given parameters.
func (db *DB) GetAllPipelines(r storage.GetAllPipelinesRequest) ([]*models.Pipeline, error) {
	bucket := db.From(r.NamespaceID)

	pipelines := []*models.Pipeline{}

	if r.Limit == 0 || r.Limit > db.maxResultsLimit {
		r.Limit = db.maxResultsLimit
	}

	err := bucket.AllByIndex("Created", &pipelines, storm.Limit(r.Limit), storm.Skip(r.Offset), storm.Reverse())
	if err != nil {
		if errors.Is(err, storm.ErrNotFound) {
			return []*models.Pipeline{}, nil
		}

		return nil, err
	}

	return pipelines, nil
}

// GetPipeline returns a single pipeline by id.
func (db *DB) GetPipeline(r storage.GetPipelineRequest) (*models.Pipeline, error) {
	bucket := db.From(r.NamespaceID)

	var pipeline models.Pipeline
	err := bucket.One("ID", r.ID, &pipeline)
	if err != nil {
		if errors.Is(err, storm.ErrNotFound) {
			return nil, storage.ErrEntityNotFound
		}

		return nil, err
	}

	return &pipeline, nil
}

func (db *DB) AddPipeline(r storage.AddPipelineRequest) error {
	tx, err := db.Begin(true)
	if err != nil {
		return err
	}
	defer tx.Rollback() // nolint: errcheck

	bucket := tx.From(r.Pipeline.Namespace)

	var pipeline models.Pipeline
	err = bucket.One("ID", r.Pipeline.ID, &pipeline)
	if errors.Is(err, storm.ErrNotFound) {
		err = bucket.Save(r.Pipeline)
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

func (db *DB) UpdatePipeline(r storage.UpdatePipelineRequest) error {
	bucket := db.From(r.Pipeline.Namespace)

	err := bucket.Update(r.Pipeline)
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
