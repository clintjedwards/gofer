package bolt

import (
	"errors"

	"github.com/asdine/storm/v3"
	"github.com/clintjedwards/gofer/internal/models"
	"github.com/clintjedwards/gofer/internal/storage"
)

func (db *DB) GetAllTriggerEvents(r storage.GetAllTriggerEventsRequest) ([]*models.TriggerEvent, error) {
	bucket := db.From(r.NamespaceID, r.PipelineID, r.PipelineTriggerLabel)

	if r.Limit == 0 || r.Limit > db.maxResultsLimit {
		r.Limit = db.maxResultsLimit
	}

	events := []*models.TriggerEvent{}
	err := bucket.AllByIndex("ID", &events, storm.Limit(r.Limit), storm.Skip(r.Offset), storm.Reverse())
	if err != nil {
		return nil, err
	}

	return events, nil
}

// GetTriggerEvent returns a single event by id
func (db *DB) GetTriggerEvent(r storage.GetTriggerEventRequest) (*models.TriggerEvent, error) {
	bucket := db.From(r.NamespaceID, r.PipelineID, r.PipelineTriggerLabel)
	var event models.TriggerEvent

	err := bucket.One("ID", r.ID, &event)
	if err != nil {
		if errors.Is(err, storm.ErrNotFound) {
			return nil, storage.ErrEntityNotFound
		}

		return nil, err
	}

	return &event, nil
}

func (db *DB) AddTriggerEvent(r storage.AddTriggerEventRequest) error {
	bucket := db.From(r.Event.NamespaceID, r.Event.PipelineID, r.Event.PipelineTriggerLabel)

	err := bucket.Save(r.Event)
	if err != nil {
		if errors.Is(err, storm.ErrNoID) {
			return storage.ErrPreconditionFailure
		}

		if errors.Is(err, storm.ErrAlreadyExists) {
			return storage.ErrEntityExists
		}

		return err
	}

	return nil
}

func (db *DB) UpdateTriggerEvent(r storage.UpdateTriggerEventRequest) error {
	bucket := db.From(r.Event.NamespaceID, r.Event.PipelineID, r.Event.PipelineTriggerLabel)
	err := bucket.Update(r.Event)
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
