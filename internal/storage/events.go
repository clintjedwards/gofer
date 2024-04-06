package storage

import (
	"database/sql"
	"errors"
	"fmt"
	"strings"

	qb "github.com/Masterminds/squirrel"
)

type Event struct {
	ID      int64
	Type    string
	Details string
	Emitted string
}

// Return all events.
// The reverse parameter allows the sorting the events in reverse chronological order (newest event first).
func (db *DB) ListEvents(conn Queryable, offset, limit int, reverse bool) ([]Event, error) {
	if limit == 0 || limit > db.maxResultsLimit {
		limit = db.maxResultsLimit
	}

	orderByStr := "id ASC"
	if reverse {
		orderByStr = "id DESC"
	}

	query, args := qb.Select("id", "type", "details", "emitted").From("events").
		OrderBy(orderByStr).
		Limit(uint64(limit)).
		Offset(uint64(offset)).
		MustSql()

	events := []Event{}
	err := conn.Select(&events, query, args...)
	if err != nil {
		return nil, fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}

	return events, nil
}

func (db *DB) InsertEvent(conn Queryable, event *Event) (int64, error) {
	result, err := qb.Insert("events").Columns("type", "details", "emitted").
		Values(event.Type, event.Details, event.Emitted).RunWith(conn).Exec()
	if err != nil {
		if strings.Contains(err.Error(), "UNIQUE constraint failed") {
			return 0, ErrEntityExists
		}

		return 0, fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}

	return result.LastInsertId()
}

func (db *DB) GetEvent(conn Queryable, id int64) (Event, error) {
	query, args := qb.Select("id", "type", "details", "emitted").
		From("events").Where(qb.Eq{"id": id}).MustSql()

	event := Event{}
	err := conn.Get(&event, query, args...)
	if err != nil {
		if errors.Is(err, sql.ErrNoRows) {
			return Event{}, ErrEntityNotFound
		}

		return Event{}, fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}

	return event, nil
}

func (db *DB) DeleteEvent(conn Queryable, id int64) error {
	_, err := qb.Delete("events").Where(qb.Eq{"id": id}).RunWith(conn).Exec()
	if err != nil {
		if errors.Is(err, sql.ErrNoRows) {
			return nil
		}

		return fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}

	return nil
}
