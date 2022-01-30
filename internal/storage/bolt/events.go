package bolt

import (
	"encoding/binary"
	"encoding/json"
	"fmt"

	"github.com/clintjedwards/gofer/internal/models"
	"github.com/clintjedwards/gofer/internal/storage"
	"go.etcd.io/bbolt"
)

const eventsBucket string = "events"

func (db *DB) GetAllEvents(r storage.GetAllEventsRequest) ([]models.Event, error) {
	if r.Reverse {
		return db.getAllEventsReverse(r.Offset, r.Limit)
	}

	return db.getAllEvents(r.Offset, r.Limit)
}

func (db *DB) getAllEvents(offset, limit int) ([]models.Event, error) {
	if limit == 0 || limit > db.maxResultsLimit {
		limit = db.maxResultsLimit
	}

	events := []models.Event{}

	err := db.Bolt.View(func(tx *bbolt.Tx) error {
		bucket := tx.Bucket([]byte(eventsBucket))

		cursor := bucket.Cursor()

		for key, value := cursor.First(); key != nil; key, value = cursor.Next() {
			if len(events) == limit {
				return nil
			}

			if btoi(key) <= int64(offset) {
				continue
			}

			var metadata models.Metadata
			err := json.Unmarshal(value, &metadata)
			if err != nil {
				return err
			}

			if metadata.Kind == "" {
				return fmt.Errorf("could not find type in event: %v", err)
			}

			switch metadata.Kind {
			case models.CreatedNamespaceEvent:
				storedEvent := &models.EventCreatedNamespace{}
				err := json.Unmarshal(value, storedEvent)
				if err != nil {
					return err
				}
				events = append(events, storedEvent)
			case models.DisabledPipelineEvent:
				storedEvent := &models.EventDisabledPipeline{}
				err := json.Unmarshal(value, storedEvent)
				if err != nil {
					return err
				}
				events = append(events, storedEvent)
			case models.EnabledPipelineEvent:
				storedEvent := &models.EventEnabledPipeline{}
				err := json.Unmarshal(value, storedEvent)
				if err != nil {
					return err
				}
				events = append(events, storedEvent)
			case models.CreatedPipelineEvent:
				storedEvent := &models.EventCreatedPipeline{}
				err := json.Unmarshal(value, storedEvent)
				if err != nil {
					return err
				}
				events = append(events, storedEvent)
			case models.StartedRunEvent:
				storedEvent := &models.EventStartedRun{}
				err := json.Unmarshal(value, storedEvent)
				if err != nil {
					return err
				}
				events = append(events, storedEvent)
			case models.CompletedRunEvent:
				storedEvent := &models.EventCompletedRun{}
				err := json.Unmarshal(value, storedEvent)
				if err != nil {
					return err
				}
				events = append(events, storedEvent)
			case models.StartedTaskRunEvent:
				storedEvent := &models.EventStartedTaskRun{}
				err := json.Unmarshal(value, storedEvent)
				if err != nil {
					return err
				}
				events = append(events, storedEvent)
			case models.ScheduledTaskRunEvent:
				storedEvent := &models.EventScheduledTaskRun{}
				err := json.Unmarshal(value, storedEvent)
				if err != nil {
					return err
				}
				events = append(events, storedEvent)
			case models.CompletedTaskRunEvent:
				storedEvent := &models.EventCompletedTaskRun{}
				err := json.Unmarshal(value, storedEvent)
				if err != nil {
					return err
				}
				events = append(events, storedEvent)
			case models.FiredTriggerEvent:
				storedEvent := &models.EventFiredTrigger{}
				err := json.Unmarshal(value, storedEvent)
				if err != nil {
					return err
				}
				events = append(events, storedEvent)
			case models.ProcessedTriggerEvent:
				storedEvent := &models.EventProcessedTrigger{}
				err := json.Unmarshal(value, storedEvent)
				if err != nil {
					return err
				}
				events = append(events, storedEvent)
			case models.ResolvedTriggerEvent:
				storedEvent := &models.EventResolvedTrigger{}
				err := json.Unmarshal(value, storedEvent)
				if err != nil {
					return err
				}
				events = append(events, storedEvent)
			}
		}
		return nil
	})
	if err != nil {
		return nil, err
	}

	return events, nil
}

func (db *DB) getAllEventsReverse(offset, limit int) ([]models.Event, error) {
	if limit == 0 || limit > db.maxResultsLimit {
		limit = db.maxResultsLimit
	}

	events := []models.Event{}

	err := db.Bolt.View(func(tx *bbolt.Tx) error {
		bucket := tx.Bucket([]byte(eventsBucket))

		cursor := bucket.Cursor()
		lastKey, _ := cursor.Last()
		// We need to check that last key isn't empty here because
		// attempting to btoi an empty byte string results in a panic
		if len(lastKey) == 0 {
			return nil
		}

		totalLength := btoi(lastKey)

		for key, value := cursor.Last(); key != nil; key, value = cursor.Prev() {
			if len(events) == limit {
				return nil
			}

			if btoi(key) > totalLength-int64(offset) {
				continue
			}

			var metadata models.Metadata
			err := json.Unmarshal(value, &metadata)
			if err != nil {
				return err
			}

			if metadata.Kind == "" {
				return fmt.Errorf("could not find type in event: %v", err)
			}

			switch metadata.Kind {
			case models.CreatedNamespaceEvent:
				storedEvent := &models.EventCreatedNamespace{}
				err := json.Unmarshal(value, storedEvent)
				if err != nil {
					return err
				}
				events = append(events, storedEvent)
			case models.DisabledPipelineEvent:
				storedEvent := &models.EventDisabledPipeline{}
				err := json.Unmarshal(value, storedEvent)
				if err != nil {
					return err
				}
				events = append(events, storedEvent)
			case models.EnabledPipelineEvent:
				storedEvent := &models.EventEnabledPipeline{}
				err := json.Unmarshal(value, storedEvent)
				if err != nil {
					return err
				}
				events = append(events, storedEvent)
			case models.CreatedPipelineEvent:
				storedEvent := &models.EventCreatedPipeline{}
				err := json.Unmarshal(value, storedEvent)
				if err != nil {
					return err
				}
				events = append(events, storedEvent)
			case models.StartedRunEvent:
				storedEvent := &models.EventStartedRun{}
				err := json.Unmarshal(value, storedEvent)
				if err != nil {
					return err
				}
				events = append(events, storedEvent)
			case models.CompletedRunEvent:
				storedEvent := &models.EventCompletedRun{}
				err := json.Unmarshal(value, storedEvent)
				if err != nil {
					return err
				}
				events = append(events, storedEvent)
			case models.StartedTaskRunEvent:
				storedEvent := &models.EventStartedTaskRun{}
				err := json.Unmarshal(value, storedEvent)
				if err != nil {
					return err
				}
				events = append(events, storedEvent)
			case models.ScheduledTaskRunEvent:
				storedEvent := &models.EventScheduledTaskRun{}
				err := json.Unmarshal(value, storedEvent)
				if err != nil {
					return err
				}
				events = append(events, storedEvent)
			case models.CompletedTaskRunEvent:
				storedEvent := &models.EventCompletedTaskRun{}
				err := json.Unmarshal(value, storedEvent)
				if err != nil {
					return err
				}
				events = append(events, storedEvent)
			case models.FiredTriggerEvent:
				storedEvent := &models.EventFiredTrigger{}
				err := json.Unmarshal(value, storedEvent)
				if err != nil {
					return err
				}
				events = append(events, storedEvent)
			case models.ProcessedTriggerEvent:
				storedEvent := &models.EventProcessedTrigger{}
				err := json.Unmarshal(value, storedEvent)
				if err != nil {
					return err
				}
				events = append(events, storedEvent)
			case models.ResolvedTriggerEvent:
				storedEvent := &models.EventResolvedTrigger{}
				err := json.Unmarshal(value, storedEvent)
				if err != nil {
					return err
				}
				events = append(events, storedEvent)
			}
		}
		return nil
	})

	return events, err
}

// GetEvent returns a single event by id.
func (db *DB) GetEvent(r storage.GetEventRequest) (models.Event, error) {
	var event models.Event

	err := db.Bolt.View(func(tx *bbolt.Tx) error {
		bucket := tx.Bucket([]byte(eventsBucket))

		eventRaw := bucket.Get(itob(r.ID))
		if eventRaw == nil {
			return storage.ErrEntityNotFound
		}

		var metadata models.Metadata
		err := json.Unmarshal(eventRaw, &metadata)
		if err != nil {
			return err
		}

		if metadata.Kind == "" {
			return fmt.Errorf("could not find type in event: %v", err)
		}

		switch metadata.Kind {
		case models.CreatedNamespaceEvent:
			storedEvent := &models.EventCreatedNamespace{}
			err := json.Unmarshal(eventRaw, storedEvent)
			if err != nil {
				return err
			}
			event = storedEvent
		case models.DisabledPipelineEvent:
			storedEvent := &models.EventDisabledPipeline{}
			err := json.Unmarshal(eventRaw, storedEvent)
			if err != nil {
				return err
			}
			event = storedEvent
		case models.EnabledPipelineEvent:
			storedEvent := &models.EventEnabledPipeline{}
			err := json.Unmarshal(eventRaw, storedEvent)
			if err != nil {
				return err
			}
			event = storedEvent
		case models.CreatedPipelineEvent:
			storedEvent := &models.EventCreatedPipeline{}
			err := json.Unmarshal(eventRaw, storedEvent)
			if err != nil {
				return err
			}
			event = storedEvent
		case models.StartedRunEvent:
			storedEvent := &models.EventStartedRun{}
			err := json.Unmarshal(eventRaw, storedEvent)
			if err != nil {
				return err
			}
			event = storedEvent
		case models.CompletedRunEvent:
			storedEvent := &models.EventCompletedRun{}
			err := json.Unmarshal(eventRaw, storedEvent)
			if err != nil {
				return err
			}
			event = storedEvent
		case models.StartedTaskRunEvent:
			storedEvent := &models.EventStartedTaskRun{}
			err := json.Unmarshal(eventRaw, storedEvent)
			if err != nil {
				return err
			}
			event = storedEvent
		case models.ScheduledTaskRunEvent:
			storedEvent := &models.EventScheduledTaskRun{}
			err := json.Unmarshal(eventRaw, storedEvent)
			if err != nil {
				return err
			}
			event = storedEvent
		case models.CompletedTaskRunEvent:
			storedEvent := &models.EventCompletedTaskRun{}
			err := json.Unmarshal(eventRaw, storedEvent)
			if err != nil {
				return err
			}
			event = storedEvent
		case models.FiredTriggerEvent:
			storedEvent := &models.EventFiredTrigger{}
			err := json.Unmarshal(eventRaw, storedEvent)
			if err != nil {
				return err
			}
			event = storedEvent
		case models.ProcessedTriggerEvent:
			storedEvent := &models.EventProcessedTrigger{}
			err := json.Unmarshal(eventRaw, storedEvent)
			if err != nil {
				return err
			}
			event = storedEvent
		case models.ResolvedTriggerEvent:
			storedEvent := &models.EventResolvedTrigger{}
			err := json.Unmarshal(eventRaw, storedEvent)
			if err != nil {
				return err
			}
			event = storedEvent
		}

		return nil
	})

	return event, err
}

func (db *DB) AddEvent(r storage.AddEventRequest) error {
	var id int64

	err := db.Bolt.Update(func(tx *bbolt.Tx) error {
		bucket := tx.Bucket([]byte(eventsBucket))

		idRaw, err := bucket.NextSequence()
		if err != nil {
			return err
		}

		id = int64(idRaw)

		r.Event.SetID(id)

		eventRaw, err := json.Marshal(r.Event)
		if err != nil {
			return err
		}

		berr := bucket.Put(itob(id), eventRaw)
		if berr != nil {
			return berr
		}

		return nil
	})

	return err
}

func (db *DB) DeleteEvent(r storage.DeleteEventRequest) error {
	err := db.Bolt.Update(func(tx *bbolt.Tx) error {
		bucket := tx.Bucket([]byte(eventsBucket))

		storedEvent := bucket.Get(itob(r.ID))
		if storedEvent == nil {
			return storage.ErrEntityNotFound
		}

		berr := bucket.Delete(itob(r.ID))
		if berr != nil {
			return berr
		}

		return nil
	})

	return err
}

// itob returns an 8-byte big endian representation of v.
func itob(v int64) []byte {
	b := make([]byte, 8)
	binary.BigEndian.PutUint64(b, uint64(v))
	return b
}

// btoi returns an int64 representation of a given byte string.
func btoi(v []byte) int64 {
	return int64(binary.BigEndian.Uint64(v))
}
