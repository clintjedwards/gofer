package storage

import (
	"database/sql"
	"errors"
	"fmt"
	"strings"

	qb "github.com/Masterminds/squirrel"
	"github.com/clintjedwards/gofer/models"
)

func (db *DB) ListObjectStoreRunKeys(namespace, pipeline string, run int64) ([]models.ObjectStoreKey, error) {
	rows, err := qb.Select("key", "created").
		From("object_store_run_keys").
		OrderBy("created ASC"). // oldest first
		Where(qb.Eq{"namespace": namespace, "pipeline": pipeline, "run": run}).RunWith(db).Query()
	if err != nil {
		return nil, fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}
	err = rows.Err()
	if err != nil {
		return nil, fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}
	defer rows.Close()

	runKeys := []models.ObjectStoreKey{}

	for rows.Next() {
		var key string
		var created int64

		err = rows.Scan(&key, &created)
		if err != nil {
			return nil, fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
		}

		runKeys = append(runKeys, models.ObjectStoreKey{
			Key:     key,
			Created: created,
		})

	}
	err = rows.Err()
	if err != nil {
		return nil, fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}

	return runKeys, nil
}

func (db *DB) InsertObjectStoreRunKey(namespace, pipeline string, run int64, objectKey *models.ObjectStoreKey) error {
	_, err := qb.Insert("object_store_run_keys").Columns("namespace", "pipeline", "run", "key", "created").Values(
		namespace, pipeline, run, objectKey.Key, objectKey.Created).RunWith(db).Exec()
	if err != nil {
		if strings.Contains(err.Error(), "UNIQUE constraint failed") {
			return ErrEntityExists
		}

		return fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}

	return nil
}

func (db *DB) DeleteObjectStoreRunKey(namespace, pipeline string, run int64, key string) error {
	_, err := qb.Delete("object_store_run_keys").
		Where(qb.Eq{"namespace": namespace, "pipeline": pipeline, "run": run, "key": key}).RunWith(db).Exec()
	if err != nil {
		if errors.Is(err, sql.ErrNoRows) {
			return nil
		}

		return fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}

	return nil
}
