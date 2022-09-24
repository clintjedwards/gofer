package storage

import (
	"database/sql"
	"errors"
	"fmt"
	"strings"

	qb "github.com/Masterminds/squirrel"
	"github.com/clintjedwards/gofer/models"
)

type UpdatableNamespaceFields struct {
	Name        *string
	Description *string
	Modified    *int64
}

func (db *DB) ListNamespaces(offset, limit int) ([]models.Namespace, error) {
	if limit == 0 || limit > db.maxResultsLimit {
		limit = db.maxResultsLimit
	}

	rows, err := qb.Select("id", "name", "description", "created", "modified").
		From("namespaces").OrderBy("id").Limit(uint64(limit)).Offset(uint64(offset)).RunWith(db).Query()
	if err != nil {
		return nil, fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}
	err = rows.Err()
	if err != nil {
		return nil, fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}
	defer rows.Close()

	namespaces := []models.Namespace{}

	for rows.Next() {
		var id string
		var name string
		var description string
		var created int64
		var modified int64

		err = rows.Scan(&id, &name, &description, &created, &modified)
		if err != nil {
			return nil, fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
		}

		namespaces = append(namespaces, models.Namespace{
			ID:          id,
			Name:        name,
			Description: description,
			Created:     created,
			Modified:    modified,
		})

	}
	err = rows.Err()
	if err != nil {
		return nil, fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}

	return namespaces, nil
}

func (db *DB) InsertNamespace(namespace *models.Namespace) error {
	_, err := qb.Insert("namespaces").Columns("id", "name", "description", "created", "modified").Values(
		namespace.ID, namespace.Name, namespace.Description, namespace.Created, namespace.Modified,
	).RunWith(db).Exec()
	if err != nil {
		if strings.Contains(err.Error(), "UNIQUE constraint failed") {
			return ErrEntityExists
		}

		return fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}

	return nil
}

func (db *DB) GetNamespace(id string) (models.Namespace, error) {
	row := qb.Select("id", "name", "description", "created", "modified").
		From("namespaces").Where(qb.Eq{"id": id}).RunWith(db).QueryRow()

	var name string
	var description string
	var created int64
	var modified int64
	err := row.Scan(&id, &name, &description, &created, &modified)
	if err != nil {
		if errors.Is(err, sql.ErrNoRows) {
			return models.Namespace{}, ErrEntityNotFound
		}

		return models.Namespace{}, fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}

	return models.Namespace{
		ID:          id,
		Name:        name,
		Description: description,
		Created:     created,
		Modified:    modified,
	}, nil
}

func (db *DB) UpdateNamespace(id string, fields UpdatableNamespaceFields) error {
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

	_, err := query.RunWith(db).Exec()
	if err != nil {
		if strings.Contains(err.Error(), "no rows in result set") {
			return ErrEntityNotFound
		}

		return fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}

	return nil
}

func (db *DB) DeleteNamespace(id string) error {
	_, err := qb.Delete("namespaces").Where(qb.Eq{"id": id}).RunWith(db).Exec()
	if err != nil {
		if errors.Is(err, sql.ErrNoRows) {
			return nil
		}

		return fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}

	return nil
}
