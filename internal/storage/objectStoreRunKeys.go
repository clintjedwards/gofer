package storage

import (
	"database/sql"
	"errors"
	"fmt"
	"strings"

	qb "github.com/Masterminds/squirrel"
)

type ObjectStoreRunKey struct {
	Namespace string
	Pipeline  string
	Run       int64
	Key       string
	Created   string
}

func (db *DB) ListObjectStoreRunKeys(conn Queryable, namespace, pipeline string, run int64) ([]ObjectStoreRunKey, error) {
	query, args := qb.Select("namespace", "pipeline", "run", "key", "created").
		From("object_store_run_keys").
		OrderBy("created ASC"). // oldest first
		Where(qb.Eq{"namespace": namespace, "pipeline": pipeline, "run": run}).MustSql()

	keys := []ObjectStoreRunKey{}
	err := conn.Select(&keys, query, args...)
	if err != nil {
		return nil, fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}

	return keys, nil
}

func (db *DB) InsertObjectStoreRunKey(conn Queryable, objectKey *ObjectStoreRunKey) error {
	_, err := qb.Insert("object_store_run_keys").
		Columns("namespace", "pipeline", "run", "key", "created").Values(
		objectKey.Namespace, objectKey.Pipeline, objectKey.Run, objectKey.Key, objectKey.Created).
		RunWith(conn).Exec()
	if err != nil {
		if strings.Contains(err.Error(), "UNIQUE constraint failed") {
			return ErrEntityExists
		}

		return fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}

	return nil
}

func (db *DB) DeleteObjectStoreRunKey(conn Queryable, namespace, pipeline string, run int64, key string) error {
	_, err := qb.Delete("object_store_run_keys").
		Where(qb.Eq{"namespace": namespace, "pipeline": pipeline, "run": run, "key": key}).RunWith(conn).Exec()
	if err != nil {
		if errors.Is(err, sql.ErrNoRows) {
			return nil
		}

		return fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}

	return nil
}
