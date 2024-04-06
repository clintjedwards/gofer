package storage

import (
	"database/sql"
	"errors"
	"fmt"
	"strings"

	qb "github.com/Masterminds/squirrel"
)

type GlobalExtensionRegistration struct {
	ID           string `db:"id"`
	Image        string `db:"image"`
	RegistryAuth string `db:"registry_auth"`
	Variables    string `db:"variables"`
	Created      string `db:"created"`
	Status       string `db:"status"`
	KeyID        string `db:"key_id"`
}

type UpdatableGlobalExtensionRegistrationFields struct {
	Image        *string
	RegistryAuth *string
	Variables    *string
	Status       *string
	KeyID        *string
}

func (db *DB) ListGlobalExtensionRegistrations(conn Queryable, offset, limit int) ([]GlobalExtensionRegistration, error) {
	if limit == 0 || limit > db.maxResultsLimit {
		limit = db.maxResultsLimit
	}

	query, args := qb.Select("id", "image", "registry_auth", "variables", "created", "status", "key_id").
		From("global_extension_registrations").
		Limit(uint64(limit)).
		Offset(uint64(offset)).MustSql()

	regs := []GlobalExtensionRegistration{}
	err := conn.Select(&regs, query, args...)
	if err != nil {
		return nil, fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}

	return regs, nil
}

func (db *DB) InsertGlobalExtensionRegistration(conn Queryable, tr *GlobalExtensionRegistration) error {
	_, err := qb.Insert("global_extension_registrations").Columns("id", "image", "registry_auth", "variables", "created",
		"status", "key_id").Values(tr.ID, tr.Image, tr.RegistryAuth, tr.Variables, tr.Created, tr.Status, tr.KeyID).
		RunWith(conn).Exec()
	if err != nil {
		if strings.Contains(err.Error(), "UNIQUE constraint failed") {
			return ErrEntityExists
		}

		return fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}

	return nil
}

func (db *DB) GetGlobalExtensionRegistration(conn Queryable, id string) (GlobalExtensionRegistration, error) {
	query, args := qb.Select("id", "image", "registry_auth", "variables", "created", "status", "key_id").
		From("global_extension_registrations").Where(qb.Eq{"id": id}).MustSql()

	reg := GlobalExtensionRegistration{}
	err := conn.Get(&reg, query, args...)
	if err != nil {
		if errors.Is(err, sql.ErrNoRows) {
			return GlobalExtensionRegistration{}, ErrEntityNotFound
		}

		return GlobalExtensionRegistration{}, fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}

	return reg, nil
}

func (db *DB) UpdateGlobalExtensionRegistration(conn Queryable, id string, fields UpdatableGlobalExtensionRegistrationFields) error {
	query := qb.Update("global_extension_registrations")

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

	if fields.KeyID != nil {
		query = query.Set("key_id", fields.KeyID)
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

func (db *DB) DeleteGlobalExtensionRegistration(conn Queryable, id string) error {
	_, err := qb.Delete("global_extension_registrations").Where(qb.Eq{"id": id}).RunWith(conn).Exec()
	if err != nil {
		if errors.Is(err, sql.ErrNoRows) {
			return nil
		}

		return fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}

	return nil
}
