package bolt

import (
	"errors"
	"time"

	"github.com/asdine/storm/v3"
	"github.com/clintjedwards/gofer/internal/models"
	"github.com/clintjedwards/gofer/internal/storage"
)

func (db *DB) GetAllRuns(r storage.GetAllRunsRequest) ([]*models.Run, error) {
	bucket := db.From(r.NamespaceID, r.PipelineID)

	if r.Limit == 0 || r.Limit > db.maxResultsLimit {
		r.Limit = db.maxResultsLimit
	}

	runs := []*models.Run{}
	err := bucket.AllByIndex("ID", &runs, storm.Limit(r.Limit), storm.Skip(r.Offset), storm.Reverse())
	if err != nil {
		return nil, err
	}

	return runs, nil
}

// GetRun returns a single Run by id
func (db *DB) GetRun(r storage.GetRunRequest) (*models.Run, error) {
	bucket := db.From(r.NamespaceID, r.PipelineID)
	var run models.Run

	err := bucket.One("ID", r.ID, &run)
	if err != nil {
		if errors.Is(err, storm.ErrNotFound) {
			return nil, storage.ErrEntityNotFound
		}

		return nil, err
	}

	return &run, nil
}

func (db *DB) AddRun(r storage.AddRunRequest) error {
	tx, err := db.Begin(true)
	if err != nil {
		return err
	}
	defer tx.Rollback() // nolint: errcheck

	pipelineBucket := tx.From(r.Run.NamespaceID)
	runBucket := tx.From(r.Run.NamespaceID, r.Run.PipelineID)

	var pipeline models.Pipeline
	err = pipelineBucket.One("ID", r.Run.PipelineID, &pipeline)
	if err != nil {
		if errors.Is(err, storm.ErrNotFound) {
			return storage.ErrEntityNotFound
		}

		return err
	}

	err = runBucket.Save(r.Run)
	if err != nil {
		return err
	}

	pipeline.LastRunID = r.Run.ID
	pipeline.LastRunTime = time.Now().UnixMilli()

	err = pipelineBucket.Update(&pipeline)
	if err != nil {
		return err
	}

	return tx.Commit()
}

func (db *DB) UpdateRun(r storage.UpdateRunRequest) error {
	bucket := db.From(r.Run.NamespaceID, r.Run.PipelineID)
	err := bucket.Update(r.Run)
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
