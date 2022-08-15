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

type UpdatableTriggerRegistrationFields struct {
	Image        *string
	RegistryAuth *models.RegistryAuth
	Variables    *map[string]string
	Status       *models.TriggerStatus
}

func (db *DB) ListTriggerRegistrations(offset, limit int) ([]models.TriggerRegistration, error) {
	if limit == 0 || limit > db.maxResultsLimit {
		limit = db.maxResultsLimit
	}

	rows, err := qb.Select("name", "image", "registry_auth", "variables", "created", "status").
		From("trigger_registrations").
		Limit(uint64(limit)).
		Offset(uint64(offset)).RunWith(db).Query()
	if err != nil {
		return nil, fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}
	defer rows.Close()

	triggerRegistrations := []models.TriggerRegistration{}

	for rows.Next() {
		triggerRegistration := models.TriggerRegistration{}

		var name string
		var image string
		var registryAuthJSON sql.NullString
		var variablesJSON string
		var created int64
		var status models.TriggerStatus

		err = rows.Scan(&name, &image, &registryAuthJSON, &variablesJSON, &created, &status)
		if err != nil {
			return nil, fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
		}

		var registryAuth *models.RegistryAuth = nil
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

		triggerRegistration.Name = name
		triggerRegistration.Image = image
		triggerRegistration.RegistryAuth = registryAuth
		triggerRegistration.Variables = variables
		triggerRegistration.Created = created
		triggerRegistration.Status = status

		triggerRegistrations = append(triggerRegistrations, triggerRegistration)
	}
	err = rows.Err()
	if err != nil {
		return nil, fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}

	return triggerRegistrations, nil
}

func (db *DB) InsertTriggerRegistration(tr *models.TriggerRegistration) error {
	variablesJSON, err := json.Marshal(tr.Variables)
	if err != nil {
		return fmt.Errorf("database error occurred; could not encode object; %v", err)
	}

	var registryAuthJSON *string = nil
	if tr.RegistryAuth != nil {
		tmpJSON, err := json.Marshal(tr.RegistryAuth)
		if err != nil {
			return fmt.Errorf("database error occurred; could not encode object; %v", err)
		}

		registryAuthJSON = ptr(string(tmpJSON))
	}

	_, err = qb.Insert("trigger_registrations").Columns("name", "image", "registry_auth", "variables", "created",
		"status").Values(tr.Name, tr.Image, registryAuthJSON, string(variablesJSON), tr.Created, tr.Status).RunWith(db).Exec()
	if err != nil {
		if strings.Contains(err.Error(), "UNIQUE constraint failed") {
			return ErrEntityExists
		}

		return fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}

	return nil
}

func (db *DB) GetTriggerRegistration(name string) (models.TriggerRegistration, error) {
	row := qb.Select("name", "image", "registry_auth", "variables", "created", "status").
		From("trigger_registrations").Where(qb.Eq{"name": name}).RunWith(db).QueryRow()

	triggerRegistration := models.TriggerRegistration{}

	var nameStr string
	var image string
	var registryAuthJSON sql.NullString
	var variablesJSON string
	var created int64
	var status models.TriggerStatus

	err := row.Scan(&nameStr, &image, &registryAuthJSON, &variablesJSON, &created, &status)
	if err != nil {
		if errors.Is(err, sql.ErrNoRows) {
			return models.TriggerRegistration{}, ErrEntityNotFound
		}

		return models.TriggerRegistration{}, fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}

	var registryAuth *models.RegistryAuth = nil
	if registryAuthJSON.Valid {
		registryAuth = &models.RegistryAuth{}
		err := json.Unmarshal([]byte(registryAuthJSON.String), registryAuth)
		if err != nil {
			return models.TriggerRegistration{}, fmt.Errorf("database error occurred; could not decode object; %v", err)
		}
	}

	variables := []models.Variable{}
	err = json.Unmarshal([]byte(variablesJSON), &variables)
	if err != nil {
		return models.TriggerRegistration{}, fmt.Errorf("database error occurred; could not decode object; %v", err)
	}

	triggerRegistration.Name = name
	triggerRegistration.Image = image
	triggerRegistration.RegistryAuth = registryAuth
	triggerRegistration.Variables = variables
	triggerRegistration.Created = created
	triggerRegistration.Status = status

	return triggerRegistration, nil
}

func (db *DB) UpdateTriggerRegistration(name string, fields UpdatableTriggerRegistrationFields) error {
	query := qb.Update("trigger_registrations")

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

	_, err := query.Where(qb.Eq{"name": name}).RunWith(db).Exec()
	if err != nil {
		if errors.Is(err, sql.ErrNoRows) {
			return ErrEntityNotFound
		}

		return fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}

	return nil
}

func (db *DB) DeleteTriggerRegistration(name string) error {
	_, err := qb.Delete("trigger_registrations").Where(qb.Eq{"name": name}).RunWith(db).Exec()
	if err != nil {
		if errors.Is(err, sql.ErrNoRows) {
			return nil
		}

		return fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}

	return nil
}
