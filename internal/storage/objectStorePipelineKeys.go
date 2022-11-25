package storage

import (
	"database/sql"
	"errors"
	"fmt"
	"strings"

	qb "github.com/Masterminds/squirrel"
)

type ObjectStorePipelineKey struct {
	Namespace string
	Pipeline  string
	Key       string
	Created   int64
}

func (db *DB) ListObjectStorePipelineKeys(conn Queryable, namespace, pipeline string) ([]ObjectStorePipelineKey, error) {
	query, args := qb.Select("namespace", "pipeline", "key", "created").
		From("object_store_pipeline_keys").
		OrderBy("created ASC"). // oldest first
		Where(qb.Eq{"namespace": namespace, "pipeline": pipeline}).MustSql()

	keys := []ObjectStorePipelineKey{}
	err := conn.Select(&keys, query, args...)
	if err != nil {
		return nil, fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}

	return keys, nil
}

func (db *DB) InsertObjectStorePipelineKey(conn Queryable, objectKey *ObjectStorePipelineKey) error {
	_, err := qb.Insert("object_store_pipeline_keys").Columns("namespace", "pipeline", "key", "created").Values(
		objectKey.Namespace, objectKey.Pipeline, objectKey.Key, objectKey.Created).RunWith(conn).Exec()
	if err != nil {
		if strings.Contains(err.Error(), "UNIQUE constraint failed") {
			return ErrEntityExists
		}

		return fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}

	return nil
}

func (db *DB) DeleteObjectStorePipelineKey(conn Queryable, namespace, pipeline, key string) error {
	_, err := qb.Delete("object_store_pipeline_keys").
		Where(qb.Eq{"namespace": namespace, "pipeline": pipeline, "key": key}).RunWith(conn).Exec()
	if err != nil {
		if errors.Is(err, sql.ErrNoRows) {
			return nil
		}

		return fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}

	return nil
}
