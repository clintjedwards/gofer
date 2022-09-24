package storage

import (
	"database/sql"
	"errors"
	"fmt"
	"strings"

	qb "github.com/Masterminds/squirrel"
	"github.com/clintjedwards/gofer/models"
)

func (db *DB) ListSecretStorePipelineKeys(namespace, pipeline string) ([]models.SecretStoreKey, error) {
	rows, err := qb.Select("key", "created").
		From("secret_store_pipeline_keys").
		Where(qb.Eq{"namespace": namespace, "pipeline": pipeline}).RunWith(db).Query()
	if err != nil {
		return nil, fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}
	err = rows.Err()
	if err != nil {
		return nil, fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}
	defer rows.Close()

	pipelineKeys := []models.SecretStoreKey{}

	for rows.Next() {
		var key string
		var created int64

		err = rows.Scan(&key, &created)
		if err != nil {
			return nil, fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
		}

		pipelineKeys = append(pipelineKeys, models.SecretStoreKey{
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

func (db *DB) GetSecretStorePipelineKey(namespace, pipeline, key string) (models.SecretStoreKey, error) {
	row := qb.Select("key", "created").
		From("secret_store_pipeline_keys").
		Where(qb.Eq{"namespace": namespace, "pipeline": pipeline, "key": key}).RunWith(db).QueryRow()

	var keyStr string
	var created int64

	err := row.Scan(&keyStr, &created)
	if err != nil {
		if errors.Is(err, sql.ErrNoRows) {
			return models.SecretStoreKey{}, ErrEntityNotFound
		}

		return models.SecretStoreKey{}, fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}

	return models.SecretStoreKey{
		Key:     key,
		Created: created,
	}, nil
}

func (db *DB) InsertSecretStorePipelineKey(namespace, pipeline string, secretKey *models.SecretStoreKey, force bool) error {
	_, err := qb.Insert("secret_store_pipeline_keys").Columns("namespace", "pipeline", "key", "created").Values(
		namespace, pipeline, secretKey.Key, secretKey.Created).RunWith(db).Exec()
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

func (db *DB) DeleteSecretStorePipelineKey(namespace, pipeline string, key string) error {
	_, err := qb.Delete("secret_store_pipeline_keys").Where(qb.Eq{"namespace": namespace, "pipeline": pipeline, "key": key}).RunWith(db).Exec()
	if err != nil {
		if errors.Is(err, sql.ErrNoRows) {
			return nil
		}

		return fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}

	return nil
}
