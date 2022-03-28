package bolt

import (
	"errors"

	"github.com/asdine/storm/v3"
	"github.com/clintjedwards/gofer/internal/config"
	"github.com/clintjedwards/gofer/internal/storage"
)

func (db *DB) GetAllTriggers(r storage.GetAllTriggersRequest) ([]*config.Trigger, error) {
	triggers := []*config.Trigger{}

	err := db.All(&triggers)
	if err != nil {
		return nil, err
	}

	return triggers, nil
}

// GetTrigger returns a single trigger by hash.
func (db *DB) GetTrigger(r storage.GetTriggerRequest) (*config.Trigger, error) {
	var trigger config.Trigger
	err := db.One("Kind", r.Kind, &trigger)
	if err != nil {
		if errors.Is(err, storm.ErrNotFound) {
			return nil, storage.ErrEntityNotFound
		}

		return nil, err
	}

	return &trigger, nil
}

func (db *DB) AddTrigger(r storage.AddTriggerRequest) error {
	tx, err := db.Begin(true)
	if err != nil {
		return err
	}
	defer tx.Rollback() // nolint: errcheck

	var trigger config.Trigger
	err = tx.One("Kind", r.Trigger.Kind, &trigger)
	if errors.Is(err, storm.ErrNotFound) {
		err = tx.Save(r.Trigger)
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

func (db *DB) DeleteTrigger(r storage.DeleteTriggerRequest) error {
	err := db.DeleteStruct(&config.Trigger{Kind: r.Kind})
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
