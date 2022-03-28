package bolt

import (
	"errors"

	"github.com/asdine/storm/v3"
	"github.com/clintjedwards/gofer/internal/config"
	"github.com/clintjedwards/gofer/internal/storage"
)

func (db *DB) GetAllNotifiers(r storage.GetAllNotifiersRequest) ([]*config.Notifier, error) {
	notifiers := []*config.Notifier{}

	err := db.All(&notifiers)
	if err != nil {
		return nil, err
	}

	return notifiers, nil
}

// GetNotifier returns a single notifier by hash.
func (db *DB) GetNotifier(r storage.GetNotifierRequest) (*config.Notifier, error) {
	var notifier config.Notifier
	err := db.One("Kind", r.Kind, &notifier)
	if err != nil {
		if errors.Is(err, storm.ErrNotFound) {
			return nil, storage.ErrEntityNotFound
		}

		return nil, err
	}

	return &notifier, nil
}

func (db *DB) AddNotifier(r storage.AddNotifierRequest) error {
	tx, err := db.Begin(true)
	if err != nil {
		return err
	}
	defer tx.Rollback() // nolint: errcheck

	var notifier config.Notifier
	err = tx.One("Kind", r.Notifier.Kind, &notifier)
	if errors.Is(err, storm.ErrNotFound) {
		err = tx.Save(r.Notifier)
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

func (db *DB) DeleteNotifier(r storage.DeleteNotifierRequest) error {
	err := db.DeleteStruct(&config.Notifier{Kind: r.Kind})
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
