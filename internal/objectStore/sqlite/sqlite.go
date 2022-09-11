package sqlite

import (
	"database/sql"
	"errors"
	"fmt"
	"strings"

	qb "github.com/Masterminds/squirrel"
	"github.com/clintjedwards/gofer/internal/objectStore"
	"github.com/jmoiron/sqlx"
	_ "github.com/mattn/go-sqlite3" // Provides sqlite3 lib
)

const initialMigration = `CREATE TABLE IF NOT EXISTS objects (
    key         TEXT    NOT NULL,
    value       BLOB    NOT NULL,
    PRIMARY KEY (key)
) STRICT;`

// DB is a representation of the datastore
type Store struct {
	*sqlx.DB
}

// New creates a new db with given settings
func New(path string) (Store, error) {
	dsn := fmt.Sprintf("%s?_journal=wal&_fk=true&_timeout=5000", path)

	db, err := sqlx.Connect("sqlite3", dsn)
	if err != nil {
		return Store{}, err
	}

	_ = db.MustExec(initialMigration)

	return Store{
		db,
	}, nil
}

func (store *Store) GetObject(key string) ([]byte, error) {
	row := qb.Select("value").
		From("objects").Where(qb.Eq{"key": key}).RunWith(store).QueryRow()

	var value []byte
	err := row.Scan(&value)
	if err != nil {
		if errors.Is(err, sql.ErrNoRows) {
			return nil, objectStore.ErrEntityNotFound
		}

		return nil, fmt.Errorf("database error occurred: %v; %w", err, objectStore.ErrInternal)
	}

	return value, nil
}

func (store *Store) PutObject(key string, content []byte, force bool) error {
	_, err := qb.Insert("objects").Columns("key", "value").Values(key, content).RunWith(store).Exec()
	if err != nil {
		if strings.Contains(err.Error(), "UNIQUE constraint failed") {
			return objectStore.ErrEntityExists
		}

		return fmt.Errorf("database error occurred: %v; %w", err, objectStore.ErrInternal)
	}

	return nil
}

func (store *Store) ListObjectKeys(prefix string) ([]string, error) {
	rows, err := qb.Select("key").
		From("objects").Where(qb.Like{"key": prefix + "%"}).RunWith(store).Query()
	if err != nil {
		return nil, fmt.Errorf("database error occurred: %v; %w", err, objectStore.ErrInternal)
	}
	defer rows.Close()

	keys := []string{}

	for rows.Next() {
		var key string

		err = rows.Scan(&key)
		if err != nil {
			return nil, fmt.Errorf("database error occurred: %v; %w", err, objectStore.ErrInternal)
		}

		keys = append(keys, key)
	}
	err = rows.Err()
	if err != nil {
		return nil, fmt.Errorf("database error occurred: %v; %w", err, objectStore.ErrInternal)
	}

	return keys, nil
}

func (store *Store) DeleteObject(key string) error {
	_, err := qb.Delete("objects").Where(qb.Eq{"key": key}).RunWith(store).Exec()
	if err != nil {
		if errors.Is(err, sql.ErrNoRows) {
			return nil
		}

		return fmt.Errorf("database error occurred: %v; %w", err, objectStore.ErrInternal)
	}

	return nil
}
