package bolt

import (
	"errors"
	"strconv"

	"github.com/asdine/storm/v3"
	"github.com/clintjedwards/gofer/internal/models"
	"github.com/clintjedwards/gofer/internal/storage"
)

func (db *DB) GetAllTaskRuns(r storage.GetAllTaskRunsRequest) ([]*models.TaskRun, error) {
	bucket := db.From(r.NamespaceID, r.PipelineID, strconv.Itoa(int(r.RunID)))

	taskRuns := []*models.TaskRun{}
	err := bucket.AllByIndex("Created", &taskRuns)
	if err != nil {
		return nil, err
	}

	return taskRuns, nil
}

// GetTaskRun returns a single TaskRun by id
func (db *DB) GetTaskRun(r storage.GetTaskRunRequest) (*models.TaskRun, error) {
	bucket := db.From(r.NamespaceID, r.PipelineID, strconv.Itoa(int(r.RunID)))

	var taskRun models.TaskRun
	err := bucket.One("ID", r.ID, &taskRun)
	if err != nil {
		if errors.Is(err, storm.ErrNotFound) {
			return nil, storage.ErrEntityNotFound
		}

		return nil, err
	}

	return &taskRun, nil
}

func (db *DB) AddTaskRun(r storage.AddTaskRunRequest) error {
	tx, err := db.Begin(true)
	if err != nil {
		return err
	}
	defer tx.Rollback() // nolint: errcheck

	runBucket := tx.From(r.TaskRun.NamespaceID, r.TaskRun.PipelineID)
	taskRunBucket := tx.From(r.TaskRun.NamespaceID, r.TaskRun.PipelineID, strconv.Itoa(int(r.TaskRun.RunID)))

	var run models.Run
	err = runBucket.One("ID", r.TaskRun.RunID, &run)
	if err != nil {
		if errors.Is(err, storm.ErrNotFound) {
			return storage.ErrEntityNotFound
		}

		return err
	}

	run.TaskRuns = append(run.TaskRuns, r.TaskRun.ID)

	err = runBucket.Update(&run)
	if err != nil {
		return err
	}

	var taskrun models.TaskRun
	err = taskRunBucket.One("ID", r.TaskRun.ID, &taskrun)
	if errors.Is(err, storm.ErrNotFound) {
		err = taskRunBucket.Save(r.TaskRun)
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

func (db *DB) UpdateTaskRun(r storage.UpdateTaskRunRequest) error {
	bucket := db.From(r.TaskRun.NamespaceID, r.TaskRun.PipelineID, strconv.Itoa(int(r.TaskRun.RunID)))
	err := bucket.Update(r.TaskRun)
	if err != nil {
		if errors.Is(err, storm.ErrIdxNotFound) {
			return storage.ErrEntityNotFound
		}

		if errors.Is(err, storm.ErrNoID) {
			return storage.ErrPreconditionFailure
		}

		return err
	}

	return nil
}
