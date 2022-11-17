package storage

import (
	"database/sql"
	"encoding/json"
	"errors"
	"fmt"
	"strings"

	qb "github.com/Masterminds/squirrel"
	"github.com/clintjedwards/gofer/models"
)

type UpdatableCommonTaskRegistrationFields struct {
	Image         *string
	RegistryAuth  *models.RegistryAuth
	Variables     *map[string]string
	Status        *models.CommonTaskStatus
	Documentation *string
}

func (db *DB) ListCommonTaskRegistrations(offset, limit int) ([]models.CommonTaskRegistration, error) {
	if limit == 0 || limit > db.maxResultsLimit {
		limit = db.maxResultsLimit
	}

	rows, err := qb.Select("name", "image", "registry_auth", "variables", "created", "status", "documentation").
		From("common_task_registrations").
		Limit(uint64(limit)).
		Offset(uint64(offset)).RunWith(db).Query()
	if err != nil {
		return nil, fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}
	err = rows.Err()
	if err != nil {
		return nil, fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}
	defer rows.Close()

	commonTaskRegistrations := []models.CommonTaskRegistration{}

	for rows.Next() {
		commonTaskRegistration := models.CommonTaskRegistration{}

		var name string
		var image string
		var registryAuthJSON sql.NullString
		var variablesJSON string
		var created int64
		var status models.CommonTaskStatus
		var documentation string

		err = rows.Scan(&name, &image, &registryAuthJSON, &variablesJSON, &created, &status, &documentation)
		if err != nil {
			return nil, fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
		}

		var registryAuth *models.RegistryAuth
		if registryAuthJSON.Valid {
			registryAuth = &models.RegistryAuth{}
			err := json.Unmarshal([]byte(registryAuthJSON.String), registryAuth)
			if err != nil {
				return nil, fmt.Errorf("database error occurred; could not decode object; %v", err)
			}
		}

		variables := []models.Variable{}
		err = json.Unmarshal([]byte(variablesJSON), &variables)
		if err != nil {
			return nil, fmt.Errorf("database error occurred; could not decode object; %v", err)
		}

		commonTaskRegistration.Name = name
		commonTaskRegistration.Image = image
		commonTaskRegistration.RegistryAuth = registryAuth
		commonTaskRegistration.Variables = variables
		commonTaskRegistration.Created = created
		commonTaskRegistration.Status = status
		commonTaskRegistration.Documentation = documentation

		commonTaskRegistrations = append(commonTaskRegistrations, commonTaskRegistration)
	}
	err = rows.Err()
	if err != nil {
		return nil, fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}

	return commonTaskRegistrations, nil
}

func (db *DB) InsertCommonTaskRegistration(tr *models.CommonTaskRegistration) error {
	variablesJSON, err := json.Marshal(tr.Variables)
	if err != nil {
		return fmt.Errorf("database error occurred; could not encode object; %v", err)
	}

	var registryAuthJSON *string
	if tr.RegistryAuth != nil {
		tmpJSON, err := json.Marshal(tr.RegistryAuth)
		if err != nil {
			return fmt.Errorf("database error occurred; could not encode object; %v", err)
		}

		registryAuthJSON = ptr(string(tmpJSON))
	}

	_, err = qb.Insert("common_task_registrations").Columns("name", "image", "registry_auth", "variables", "created",
		"status", "documentation").Values(tr.Name, tr.Image, registryAuthJSON, string(variablesJSON), tr.Created, tr.Status, tr.Documentation).RunWith(db).Exec()
	if err != nil {
		if strings.Contains(err.Error(), "UNIQUE constraint failed") {
			return ErrEntityExists
		}

		return fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}

	return nil
}

func (db *DB) GetCommonTaskRegistration(name string) (models.CommonTaskRegistration, error) {
	row := qb.Select("name", "image", "registry_auth", "variables", "created", "status", "documentation").
		From("common_task_registrations").Where(qb.Eq{"name": name}).RunWith(db).QueryRow()

	commonTaskRegistration := models.CommonTaskRegistration{}

	var nameStr string
	var image string
	var registryAuthJSON sql.NullString
	var variablesJSON string
	var created int64
	var status models.CommonTaskStatus
	var documentation string

	err := row.Scan(&nameStr, &image, &registryAuthJSON, &variablesJSON, &created, &status, &documentation)
	if err != nil {
		if errors.Is(err, sql.ErrNoRows) {
			return models.CommonTaskRegistration{}, ErrEntityNotFound
		}

		return models.CommonTaskRegistration{}, fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}

	var registryAuth *models.RegistryAuth
	if registryAuthJSON.Valid {
		registryAuth = &models.RegistryAuth{}
		err := json.Unmarshal([]byte(registryAuthJSON.String), registryAuth)
		if err != nil {
			return models.CommonTaskRegistration{}, fmt.Errorf("database error occurred; could not decode object; %v", err)
		}
	}

	variables := []models.Variable{}
	err = json.Unmarshal([]byte(variablesJSON), &variables)
	if err != nil {
		return models.CommonTaskRegistration{}, fmt.Errorf("database error occurred; could not decode object; %v", err)
	}

	commonTaskRegistration.Name = name
	commonTaskRegistration.Image = image
	commonTaskRegistration.RegistryAuth = registryAuth
	commonTaskRegistration.Variables = variables
	commonTaskRegistration.Created = created
	commonTaskRegistration.Status = status
	commonTaskRegistration.Documentation = documentation

	return commonTaskRegistration, nil
}

func (db *DB) UpdateCommonTaskRegistration(name string, fields UpdatableCommonTaskRegistrationFields) error {
	query := qb.Update("common_task_registrations")

	if fields.Image != nil {
		query = query.Set("image", fields.Image)
	}

	if fields.RegistryAuth != nil {
		registryAuth, err := json.Marshal(fields.RegistryAuth)
		if err != nil {
			return fmt.Errorf("database error occurred; could not encode object; %v", err)
		}
		query = query.Set("registry_auth", registryAuth)
	}

	if fields.Variables != nil {
		variables, err := json.Marshal(fields.Variables)
		if err != nil {
			return fmt.Errorf("database error occurred; could not encode object; %v", err)
		}
		query = query.Set("variables", variables)
	}

	if fields.Status != nil {
		query = query.Set("status", fields.Status)
	}

	if fields.Documentation != nil {
		query = query.Set("documentation", fields.Documentation)
	}

	_, err := query.Where(qb.Eq{"name": name}).RunWith(db).Exec()
	if err != nil {
		if errors.Is(err, sql.ErrNoRows) {
			return ErrEntityNotFound
		}

		return fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}

	return nil
}

func (db *DB) DeleteCommonTaskRegistration(name string) error {
	_, err := qb.Delete("common_task_registrations").Where(qb.Eq{"name": name}).RunWith(db).Exec()
	if err != nil {
		if errors.Is(err, sql.ErrNoRows) {
			return nil
		}

		return fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}

	return nil
}
