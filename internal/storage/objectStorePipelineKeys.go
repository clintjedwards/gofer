package storage

import (
	"database/sql"
	"errors"
	"fmt"
	"strings"

	qb "github.com/Masterminds/squirrel"
	"github.com/clintjedwards/gofer/models"
)

func (db *DB) ListObjectStorePipelineKeys(namespace, pipeline string) ([]models.ObjectStoreKey, error) {
	rows, err := qb.Select("key", "created").
		From("object_store_pipeline_keys").
		OrderBy("created ASC"). // oldest first
		Where(qb.Eq{"namespace": namespace, "pipeline": pipeline}).RunWith(db).Query()
	if err != nil {
		return nil, fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}
	defer rows.Close()

	pipelineKeys := []models.ObjectStoreKey{}

	for rows.Next() {
		var key string
		var created int64

		err = rows.Scan(&key, &created)
		if err != nil {
			return nil, fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
		}

		pipelineKeys = append(pipelineKeys, models.ObjectStoreKey{
			Key:     key,
			Created: created,
		})

	}
	err = rows.Err()
	if err != nil {
		return nil, fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}

	return pipelineKeys, nil
}

func (db *DB) InsertObjectStorePipelineKey(namespace, pipeline string, objectKey *models.ObjectStoreKey) error {
	_, err := qb.Insert("object_store_pipeline_keys").Columns("namespace", "pipeline", "key", "created").Values(
		namespace, pipeline, objectKey.Key, objectKey.Created).RunWith(db).Exec()
	if err != nil {
		if strings.Contains(err.Error(), "UNIQUE constraint failed") {
			return ErrEntityExists
		}

		return fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}

	return nil
}

func (db *DB) DeleteObjectStorePipelineKey(namespace, pipeline string, key string) error {
	_, err := qb.Delete("object_store_pipeline_keys").Where(qb.Eq{"namespace": namespace, "pipeline": pipeline, "key": key}).RunWith(db).Exec()
	if err != nil {
		if errors.Is(err, sql.ErrNoRows) {
			return nil
		}

		return fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}

	return nil
}
