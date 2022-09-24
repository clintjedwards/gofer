package storage

import (
	"database/sql"
	"encoding/json"
	"errors"
	"fmt"
	"strings"

	qb "github.com/Masterminds/squirrel"
	"github.com/clintjedwards/gofer/models"
)

// Return all events; limited to 200 rows in any one response.
// The reverse parameter allows the sorting the events in reverse chronological order (newest event first).
func (db *DB) ListEvents(offset, limit int, reverse bool) ([]models.Event, error) {
	if limit == 0 || limit > db.maxResultsLimit {
		limit = db.maxResultsLimit
	}

	orderByStr := "id ASC"
	if reverse {
		orderByStr = "id DESC"
	}

	rows, err := qb.Select("id", "kind", "details", "emitted").From("events").
		OrderBy(orderByStr).
		Limit(uint64(limit)).
		Offset(uint64(offset)).RunWith(db).Query()
	if err != nil {
		return nil, fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}
	err = rows.Err()
	if err != nil {
		return nil, fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}
	defer rows.Close()

	events := []models.Event{}

	for rows.Next() {
		event := models.Event{}

		var id int64
		var kind string
		var detailsJSON string
		var emitted int64

		err = rows.Scan(&id, &kind, &detailsJSON, &emitted)
		if err != nil {
			return nil, fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
		}

		// TODO(clintjedwards): There is a known data race condition here that is safe
		// but the go data race detector will squawk. What happens is the interface
		// object below gets read in and passed to the calling function.
		// That function will usually then pass that reference to other functions
		// and then at some point read. This is technically a data race since the memory
		// is access in a read/write fashion in two separate threads, but not an issue
		// for us since we only do one read/write cycle and that write can only occur
		// before any reads.
		details := models.EventKindMap[models.EventKind(kind)]
		err := json.Unmarshal([]byte(detailsJSON), &details)
		if err != nil {
			return nil, fmt.Errorf("database error occurred; could not decode object; %v", err)
		}

		event.ID = id
		event.Kind = models.EventKind(kind)
		event.Details = details
		event.Emitted = emitted

		events = append(events, event)
	}
	err = rows.Err()
	if err != nil {
		return nil, fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}

	return events, nil
}

func (db *DB) InsertEvent(event *models.Event) (int64, error) {
	detailsJSON, err := json.Marshal(event.Details)
	if err != nil {
		return 0, fmt.Errorf("database error occurred; could not encode object; %v", err)
	}

	result, err := qb.Insert("events").Columns("kind", "details", "emitted").
		Values(event.Kind, string(detailsJSON), event.Emitted).RunWith(db).Exec()
	if err != nil {
		if strings.Contains(err.Error(), "UNIQUE constraint failed") {
			return 0, ErrEntityExists
		}

		return 0, fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}

	return result.LastInsertId()
}

func (db *DB) GetEvent(id int64) (models.Event, error) {
	row := qb.Select("id", "kind", "details", "emitted").From("events").Where(qb.Eq{"id": id}).RunWith(db).QueryRow()

	var eventID int64
	var kind string
	var detailsJSON string
	var emitted int64

	err := row.Scan(&eventID, &kind, &detailsJSON, &emitted)
	if err != nil {
		if errors.Is(err, sql.ErrNoRows) {
			return models.Event{}, ErrEntityNotFound
		}

		return models.Event{}, fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}

	details := models.EventKindMap[models.EventKind(kind)]
	err = json.Unmarshal([]byte(detailsJSON), &details)
	if err != nil {
		return models.Event{}, fmt.Errorf("database error occurred; could not decode object; %v", err)
	}

	retrievedEvent := models.Event{}

	retrievedEvent.ID = eventID
	retrievedEvent.Kind = models.EventKind(kind)
	retrievedEvent.Details = details
	retrievedEvent.Emitted = emitted

	return retrievedEvent, nil
}

func (db *DB) DeleteEvent(id int64) error {
	_, err := qb.Delete("events").Where(qb.Eq{"id": id}).RunWith(db).Exec()
	if err != nil {
		if errors.Is(err, sql.ErrNoRows) {
			return nil
		}

		return fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}

	return nil
}
