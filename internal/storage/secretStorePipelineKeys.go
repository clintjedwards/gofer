package storage

import (
	"database/sql"
	"errors"
	"fmt"
	"strings"

	qb "github.com/Masterminds/squirrel"
)

type SecretStorePipelineKey struct {
	Namespace string
	Pipeline  string
	Key       string
	Created   int64
}

func (db *DB) ListSecretStorePipelineKeys(conn Queryable, namespace, pipeline string) ([]SecretStorePipelineKey, error) {
	query, args := qb.Select("namespace", "pipeline", "key", "created").
		From("secret_store_pipeline_keys").
		Where(qb.Eq{"namespace": namespace, "pipeline": pipeline}).MustSql()

	keys := []SecretStorePipelineKey{}
	err := conn.Select(&keys, query, args...)
	if err != nil {
		return nil, fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}

	return keys, nil
}

func (db *DB) GetSecretStorePipelineKey(conn Queryable, namespace, pipeline, key string) (SecretStorePipelineKey, error) {
	query, args := qb.Select("namespace", "pipeline", "key", "created").
		From("secret_store_pipeline_keys").
		Where(qb.Eq{"namespace": namespace, "pipeline": pipeline, "key": key}).MustSql()

	secretKey := SecretStorePipelineKey{}
	err := conn.Get(&secretKey, query, args...)
	if err != nil {
		if errors.Is(err, sql.ErrNoRows) {
			return SecretStorePipelineKey{}, ErrEntityNotFound
		}

		return SecretStorePipelineKey{}, fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}

	return secretKey, nil
}

func (db *DB) InsertSecretStorePipelineKey(conn Queryable, secretKey *SecretStorePipelineKey, force bool,
) error {
	_, err := qb.Insert("secret_store_pipeline_keys").Columns("namespace", "pipeline", "key", "created").Values(
		secretKey.Namespace, secretKey.Pipeline, secretKey.Key, secretKey.Created).RunWith(conn).Exec()
	if err != nil {
		if strings.Contains(err.Error(), "UNIQUE constraint failed") && !force {
			return ErrEntityExists
		}

		// We should update the key's created if the flag for force was passed down.
		if strings.Contains(err.Error(), "UNIQUE constraint failed") {
			_, err = qb.Update("secret_store_pipeline_keys").Set("created", secretKey.Created).RunWith(db).Exec()
			if err != nil {
				return err
			}
			return nil
		}

		return fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}

	return nil
}

func (db *DB) DeleteSecretStorePipelineKey(conn Queryable, namespace, pipeline, key string) error {
	_, err := qb.Delete("secret_store_pipeline_keys").Where(qb.Eq{
		"namespace": namespace, "pipeline": pipeline, "key": key,
	}).RunWith(conn).Exec()
	if err != nil {
		if errors.Is(err, sql.ErrNoRows) {
			return nil
		}

		return fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}

	return nil
}
