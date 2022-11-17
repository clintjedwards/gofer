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

type UpdatableRunFields struct {
	Ended               *int64
	State               *models.RunState
	Status              *models.RunStatus
	StatusReason        *models.RunStatusReason
	TaskRuns            *[]string
	Variables           *[]models.Variable
	StoreObjectsExpired *bool
}

func (db *DB) ListRuns(conn qb.BaseRunner, offset, limit int, namespace, pipeline string) ([]models.Run, error) {
	if conn == nil {
		conn = db
	}

	if limit == 0 || limit > db.maxResultsLimit {
		limit = db.maxResultsLimit
	}

	rows, err := qb.Select("namespace", "pipeline", "id", "started", "ended", "state", "status", "status_reason",
		"task_runs", "trigger", "variables", "store_objects_expired").
		From("runs").
		Where(qb.Eq{"namespace": namespace, "pipeline": pipeline}).
		OrderBy("started DESC").
		Limit(uint64(limit)).
		Offset(uint64(offset)).RunWith(conn).Query()
	if err != nil {
		return nil, fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}
	err = rows.Err()
	if err != nil {
		return nil, fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}
	defer rows.Close()

	runs := []models.Run{}

	for rows.Next() {
		run := models.Run{}

		var namespace string
		var pipeline string
		var id int64
		var started int64
		var ended int64
		var state string
		var status string
		var statusReasonJSON sql.NullString
		var taskRunsJSON string
		var triggerJSON string
		var variablesJSON string
		var storeObjectsExpired bool

		err = rows.Scan(&namespace, &pipeline, &id, &started, &ended, &state, &status, &statusReasonJSON,
			&taskRunsJSON, &triggerJSON, &variablesJSON, &storeObjectsExpired)
		if err != nil {
			return nil, fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
		}

		var statusReason *models.RunStatusReason
		if statusReasonJSON.Valid {
			statusReason = &models.RunStatusReason{}
			err := json.Unmarshal([]byte(statusReasonJSON.String), statusReason)
			if err != nil {
				return nil, fmt.Errorf("database error occurred; could not decode object; %v", err)
			}
		}

		taskRuns := []string{}
		err = json.Unmarshal([]byte(taskRunsJSON), &taskRuns)
		if err != nil {
			return nil, fmt.Errorf("database error occurred; could not decode object; %v", err)
		}

		trigger := models.TriggerInfo{}
		err = json.Unmarshal([]byte(triggerJSON), &trigger)
		if err != nil {
			return nil, fmt.Errorf("database error occurred; could not decode object; %v", err)
		}

		variables := []models.Variable{}
		err = json.Unmarshal([]byte(variablesJSON), &variables)
		if err != nil {
			return nil, fmt.Errorf("database error occurred; could not decode object; %v", err)
		}

		run.Namespace = namespace
		run.Pipeline = pipeline
		run.ID = id
		run.Started = started
		run.Ended = ended
		run.State = models.RunState(state)
		run.Status = models.RunStatus(status)
		run.StatusReason = statusReason
		run.TaskRuns = taskRuns
		run.Trigger = trigger
		run.Variables = variables
		run.StoreObjectsExpired = storeObjectsExpired

		runs = append(runs, run)
	}
	err = rows.Err()
	if err != nil {
		return nil, fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}

	return runs, nil
}

func (db *DB) InsertRun(run *models.Run) (int64, error) {
	tx, err := db.Begin()
	if err != nil {
		mustRollback(tx)
		return 0, err
	}

	var statusReasonJSON *string
	if run.StatusReason != nil {
		rawJSON, err := json.Marshal(run.StatusReason)
		if err != nil {
			return 0, fmt.Errorf("database error occurred; could not encode object; %v", err)
		}

		statusReasonJSON = ptr(string(rawJSON))
	}

	taskRunsJSON, err := json.Marshal(run.TaskRuns)
	if err != nil {
		return 0, fmt.Errorf("database error occurred; could not encode object; %v", err)
	}

	triggerJSON, err := json.Marshal(run.Trigger)
	if err != nil {
		return 0, fmt.Errorf("database error occurred; could not encode object; %v", err)
	}

	variablesJSON, err := json.Marshal(run.Variables)
	if err != nil {
		return 0, fmt.Errorf("database error occurred; could not encode object; %v", err)
	}

	lastRun, err := db.ListRuns(tx, 0, 1, run.Namespace, run.Pipeline)
	if err != nil {
		mustRollback(tx)
		return 0, fmt.Errorf("database error occurred; could not encode object; %v", err)
	}

	var nextID int64 = 1

	if len(lastRun) != 0 {
		nextID = lastRun[0].ID + 1
	}

	_, err = qb.Insert("runs").Columns("namespace", "pipeline", "id", "started", "ended", "state", "status",
		"status_reason", "task_runs", "trigger", "variables", "store_objects_expired").Values(
		run.Namespace, run.Pipeline, nextID, run.Started, run.Ended, run.State, run.Status, statusReasonJSON,
		string(taskRunsJSON), string(triggerJSON), string(variablesJSON), run.StoreObjectsExpired,
	).RunWith(tx).Exec()
	if err != nil {
		if strings.Contains(err.Error(), "UNIQUE constraint failed") {
			return 0, ErrEntityExists
		}

		return 0, fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}

	err = tx.Commit()
	if err != nil {
		mustRollback(tx)
		return 0, fmt.Errorf("database error occurred; could not commit; %v", err)
	}

	return nextID, nil
}

func (db *DB) GetRun(namespace, pipeline string, run int64) (models.Run, error) {
	row := qb.Select("started", "ended", "state", "status", "status_reason", "task_runs", "trigger", "variables",
		"store_objects_expired").From("runs").
		Where(qb.Eq{"namespace": namespace, "pipeline": pipeline, "id": run}).RunWith(db).QueryRow()

	var started int64
	var ended int64
	var state string
	var status string
	var statusReasonJSON sql.NullString
	var taskRunsJSON string
	var triggerJSON string
	var variablesJSON string
	var storeObjectsExpired bool

	err := row.Scan(&started, &ended, &state, &status, &statusReasonJSON, &taskRunsJSON, &triggerJSON,
		&variablesJSON, &storeObjectsExpired)
	if err != nil {
		if errors.Is(err, sql.ErrNoRows) {
			return models.Run{}, ErrEntityNotFound
		}

		return models.Run{}, fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}

	var statusReason *models.RunStatusReason
	if statusReasonJSON.Valid {
		statusReason = &models.RunStatusReason{}
		err := json.Unmarshal([]byte(statusReasonJSON.String), statusReason)
		if err != nil {
			return models.Run{}, fmt.Errorf("database error occurred; could not decode object; %v", err)
		}
	}

	taskRuns := []string{}
	err = json.Unmarshal([]byte(taskRunsJSON), &taskRuns)
	if err != nil {
		return models.Run{}, fmt.Errorf("database error occurred; could not decode object; %v", err)
	}

	trigger := models.TriggerInfo{}
	err = json.Unmarshal([]byte(triggerJSON), &trigger)
	if err != nil {
		return models.Run{}, fmt.Errorf("database error occurred; could not decode object; %v", err)
	}

	variables := []models.Variable{}
	err = json.Unmarshal([]byte(variablesJSON), &variables)
	if err != nil {
		return models.Run{}, fmt.Errorf("database error occurred; could not decode object; %v", err)
	}

	retrievedRun := models.Run{}

	retrievedRun.Namespace = namespace
	retrievedRun.Pipeline = pipeline
	retrievedRun.ID = run
	retrievedRun.Started = started
	retrievedRun.Ended = ended
	retrievedRun.State = models.RunState(state)
	retrievedRun.Status = models.RunStatus(status)
	retrievedRun.StatusReason = statusReason
	retrievedRun.TaskRuns = taskRuns
	retrievedRun.Trigger = trigger
	retrievedRun.Variables = variables
	retrievedRun.StoreObjectsExpired = storeObjectsExpired

	return retrievedRun, nil
}

func (db *DB) UpdateRun(namespace, pipeline string, run int64, fields UpdatableRunFields) error {
	query := qb.Update("runs")

	if fields.Ended != nil {
		query = query.Set("ended", fields.Ended)
	}

	if fields.State != nil {
		query = query.Set("state", fields.State)
	}

	if fields.Status != nil {
		query = query.Set("status", fields.Status)
	}

	if fields.StatusReason != nil {
		statusReason, err := json.Marshal(fields.StatusReason)
		if err != nil {
			return fmt.Errorf("database error occurred; could not encode object; %v", err)
		}
		query = query.Set("status_reason", string(statusReason))
	}

	if fields.TaskRuns != nil {
		taskRuns, err := json.Marshal(fields.TaskRuns)
		if err != nil {
			return fmt.Errorf("database error occurred; could not encode object; %v", err)
		}
		query = query.Set("task_runs", taskRuns)
	}

	if fields.Variables != nil {
		variables, err := json.Marshal(fields.Variables)
		if err != nil {
			return fmt.Errorf("database error occurred; could not encode object; %v", err)
		}
		query = query.Set("variables", variables)
	}

	if fields.StoreObjectsExpired != nil {
		query = query.Set("store_objects_expired", fields.StoreObjectsExpired)
	}

	_, err := query.Where(qb.Eq{"namespace": namespace, "pipeline": pipeline, "id": run}).RunWith(db).Exec()
	if err != nil {
		if errors.Is(err, sql.ErrNoRows) {
			return ErrEntityNotFound
		}

		return fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}

	return nil
}

func (db *DB) DeleteRun(namespace, pipeline string, id int64) error {
	_, err := qb.Delete("runs").Where(qb.Eq{"namespace": namespace, "pipeline": pipeline, "id": id}).RunWith(db).Exec()
	if err != nil {
		if errors.Is(err, sql.ErrNoRows) {
			return nil
		}

		return fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}

	return nil
}
