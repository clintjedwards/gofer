package storage

import (
	"database/sql"
	"errors"
	"fmt"
	"strings"

	qb "github.com/Masterminds/squirrel"
)

type CommonTaskRegistration struct {
	Name          string
	Image         string
	RegistryAuth  string `db:"registry_auth"`
	Variables     string
	Created       int64
	Status        string
	Documentation string
}

type UpdatableCommonTaskRegistrationFields struct {
	Image         *string
	RegistryAuth  *string
	Variables     *string
	Status        *string
	Documentation *string
}

func (db *DB) ListCommonTaskRegistrations(conn Queryable, offset, limit int) ([]CommonTaskRegistration, error) {
	if limit == 0 || limit > db.maxResultsLimit {
		limit = db.maxResultsLimit
	}

	query, args := qb.Select("name", "image", "registry_auth", "variables", "created", "status", "documentation").
		From("global_common_task_registrations").
		Limit(uint64(limit)).
		Offset(uint64(offset)).MustSql()

	tasks := []CommonTaskRegistration{}
	err := conn.Select(&tasks, query, args...)
	if err != nil {
		return nil, fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}

	return tasks, nil
}

func (db *DB) InsertCommonTaskRegistration(conn Queryable, tr *CommonTaskRegistration) error {
	_, err := qb.Insert("global_common_task_registrations").
		Columns("name", "image", "registry_auth", "variables", "created", "status", "documentation").
		Values(tr.Name, tr.Image, tr.RegistryAuth, tr.Variables, tr.Created, tr.Status, tr.Documentation).
		RunWith(conn).Exec()
	if err != nil {
		if strings.Contains(err.Error(), "UNIQUE constraint failed") {
			return ErrEntityExists
		}

		return fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}

	return nil
}

func (db *DB) GetCommonTaskRegistration(conn Queryable, name string) (CommonTaskRegistration, error) {
	query, args := qb.Select("name", "image", "registry_auth", "variables", "created", "status", "documentation").
		From("global_common_task_registrations").Where(qb.Eq{"name": name}).MustSql()

	task := CommonTaskRegistration{}
	err := conn.Get(&task, query, args...)
	if err != nil {
		if errors.Is(err, sql.ErrNoRows) {
			return CommonTaskRegistration{}, ErrEntityNotFound
		}

		return CommonTaskRegistration{}, fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}

	return task, nil
}

func (db *DB) UpdateCommonTaskRegistration(conn Queryable, name string, fields UpdatableCommonTaskRegistrationFields) error {
	query := qb.Update("global_common_task_registrations")

	if fields.Image != nil {
		query = query.Set("image", fields.Image)
	}

	if fields.RegistryAuth != nil {
		query = query.Set("registry_auth", fields.RegistryAuth)
	}

	if fields.Variables != nil {
		query = query.Set("variables", fields.Variables)
	}

	if fields.Status != nil {
		query = query.Set("status", fields.Status)
	}

	if fields.Documentation != nil {
		query = query.Set("documentation", fields.Documentation)
	}

	_, err := query.Where(qb.Eq{"name": name}).RunWith(conn).Exec()
	if err != nil {
		if errors.Is(err, sql.ErrNoRows) {
			return ErrEntityNotFound
		}

		return fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}

	return nil
}

func (db *DB) DeleteCommonTaskRegistration(conn Queryable, name string) error {
	_, err := qb.Delete("common_task_registrations").Where(qb.Eq{"name": name}).RunWith(conn).Exec()
	if err != nil {
		if errors.Is(err, sql.ErrNoRows) {
			return nil
		}

		return fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}

	return nil
}
