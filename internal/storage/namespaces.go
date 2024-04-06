package storage

import (
	"database/sql"
	"errors"
	"fmt"
	"strings"

	qb "github.com/Masterminds/squirrel"
)

type Namespace struct {
	ID          string
	Name        string
	Description string
	Created     string
	Modified    string
}

type UpdatableNamespaceFields struct {
	Name        *string
	Description *string
	Modified    *string
}

func (db *DB) ListNamespaces(conn Queryable, offset, limit int) ([]Namespace, error) {
	if limit == 0 || limit > db.maxResultsLimit {
		limit = db.maxResultsLimit
	}

	query, args := qb.Select("id", "name", "description", "created", "modified").
		From("namespaces").OrderBy("id").Limit(uint64(limit)).Offset(uint64(offset)).MustSql()

	namespaces := []Namespace{}
	err := conn.Select(&namespaces, query, args...)
	if err != nil {
		return nil, fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}

	return namespaces, nil
}

func (db *DB) InsertNamespace(conn Queryable, namespace *Namespace) error {
	_, err := qb.Insert("namespaces").Columns("id", "name", "description", "created", "modified").Values(
		namespace.ID, namespace.Name, namespace.Description, namespace.Created, namespace.Modified,
	).RunWith(conn).Exec()
	if err != nil {
		if strings.Contains(err.Error(), "UNIQUE constraint failed") {
			return ErrEntityExists
		}

		return fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}

	return nil
}

func (db *DB) GetNamespace(conn Queryable, id string) (Namespace, error) {
	query, args := qb.Select("id", "name", "description", "created", "modified").
		From("namespaces").Where(qb.Eq{"id": id}).MustSql()

	namespace := Namespace{}
	err := conn.Get(&namespace, query, args...)
	if err != nil {
		if errors.Is(err, sql.ErrNoRows) {
			return Namespace{}, ErrEntityNotFound
		}

		return Namespace{}, fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}

	return namespace, nil
}

func (db *DB) UpdateNamespace(conn Queryable, id string, fields UpdatableNamespaceFields) error {
	query := qb.Update("namespaces")

	if fields.Name != nil {
		query = query.Set("name", fields.Name)
	}

	if fields.Description != nil {
		query = query.Set("description", fields.Description)
	}

	if fields.Modified != nil {
		query = query.Set("modified", fields.Modified)
	}

	_, err := query.Where(qb.Eq{"id": id}).RunWith(conn).Exec()
	if err != nil {
		if errors.Is(err, sql.ErrNoRows) {
			return ErrEntityNotFound
		}

		return fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}

	return nil
}

func (db *DB) DeleteNamespace(conn Queryable, id string) error {
	_, err := qb.Delete("namespaces").Where(qb.Eq{"id": id}).RunWith(conn).Exec()
	if err != nil {
		if errors.Is(err, sql.ErrNoRows) {
			return nil
		}

		return fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}

	return nil
}
