package storage

import (
	"database/sql"
	"errors"
	"fmt"
	"strings"

	qb "github.com/Masterminds/squirrel"
)

type SecretStoreGlobalKey struct {
	Key            string `db:"key"`
	Namespaces     string `db:"namespaces"`
	ExtensionsOnly bool   `db:"extensions_only"`
	Created        int64  `db:"created"`
}

type UpdatableSecretStoreGlobalKeyFields struct {
	Namespaces     *string
	ExtensionsOnly *bool
}

func (db *DB) ListSecretStoreGlobalKeys(conn Queryable) ([]SecretStoreGlobalKey, error) {
	query, args := qb.Select("key", "namespaces", "extensions_only", "created").From("secret_store_global_keys").MustSql()

	keys := []SecretStoreGlobalKey{}
	err := conn.Select(&keys, query, args...)
	if err != nil {
		return nil, fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}

	return keys, nil
}

func (db *DB) GetSecretStoreGlobalKey(conn Queryable, key string) (SecretStoreGlobalKey, error) {
	query, args := qb.Select("key", "namespaces", "extensions_only", "created").From("secret_store_global_keys").Where(qb.Eq{"key": key}).MustSql()

	secretKey := SecretStoreGlobalKey{}
	err := conn.Get(&secretKey, query, args...)
	if err != nil {
		if errors.Is(err, sql.ErrNoRows) {
			return SecretStoreGlobalKey{}, ErrEntityNotFound
		}

		return SecretStoreGlobalKey{}, fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}

	return secretKey, nil
}

func (db *DB) InsertSecretStoreGlobalKey(conn Queryable, secretKey *SecretStoreGlobalKey, force bool) error {
	_, err := qb.Insert("secret_store_global_keys").Columns("key", "namespaces", "extensions_only", "created").
		Values(secretKey.Key, secretKey.Namespaces, secretKey.ExtensionsOnly, secretKey.Created).RunWith(conn).Exec()
	if err != nil {
		if strings.Contains(err.Error(), "UNIQUE constraint failed") && !force {
			return ErrEntityExists
		}

		// We should update the key's created if the flag for force was passed down.
		if strings.Contains(err.Error(), "UNIQUE constraint failed") {
			_, err = qb.Update("secret_store_pipeline_keys").Set("created", secretKey.Created).RunWith(conn).Exec()
			if err != nil {
				return err
			}
			return nil
		}

		return fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}

	return nil
}

func (db *DB) UpdateSecretStoreGlobalKey(conn Queryable, key string, fields UpdatableSecretStoreGlobalKeyFields) error {
	query := qb.Update("secret_store_global_keys")

	if fields.Namespaces != nil {
		query = query.Set("namespaces", fields.Namespaces)
	}

	if fields.ExtensionsOnly != nil {
		query = query.Set("extensions_only", fields.ExtensionsOnly)
	}

	_, err := query.Where(qb.Eq{"key": key}).RunWith(conn).Exec()
	if err != nil {
		if errors.Is(err, sql.ErrNoRows) {
			return ErrEntityNotFound
		}

		return fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}

	return nil
}

func (db *DB) DeleteSecretStoreGlobalKey(conn Queryable, key string) error {
	_, err := qb.Delete("secret_store_global_keys").Where(qb.Eq{"key": key}).RunWith(conn).Exec()
	if err != nil {
		if errors.Is(err, sql.ErrNoRows) {
			return nil
		}

		return fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}

	return nil
}
