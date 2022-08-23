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

type UpdatableTaskRunFields struct {
	Started      *int64
	Ended        *int64
	ExitCode     *int64
	State        *models.TaskRunState
	Status       *models.TaskRunStatus
	StatusReason *models.TaskRunStatusReason
	LogsExpired  *bool
	LogsRemoved  *bool
	Variables    *[]models.Variable
}

func (db *DB) ListTaskRuns(offset, limit int, namespace, pipeline string, run int64) ([]models.TaskRun, error) {
	if limit == 0 || limit > db.maxResultsLimit {
		limit = db.maxResultsLimit
	}

	rows, err := qb.Select("namespace", "pipeline", "run", "id", "task", "created", "started", "ended", "exit_code",
		"state", "status", "status_reason", "logs_expired", "logs_removed", "variables").
		From("task_runs").
		Where(qb.Eq{"namespace": namespace, "pipeline": pipeline, "run": run}).
		Limit(uint64(limit)).
		Offset(uint64(offset)).RunWith(db).Query()
	if err != nil {
		return nil, fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}
	defer rows.Close()

	taskRuns := []models.TaskRun{}

	for rows.Next() {
		taskRun := models.TaskRun{}

		var namespace string
		var pipeline string
		var run int64
		var id string
		var taskJSON string
		var created int64
		var started int64
		var ended int64
		var exitCodeRaw sql.NullInt64
		var state string
		var status string
		var statusReasonJSON sql.NullString
		var logsExpired bool
		var logsRemoved bool
		var variablesJSON string

		err = rows.Scan(&namespace, &pipeline, &run, &id, &taskJSON, &created, &started, &ended,
			&exitCodeRaw, &state, &status, &statusReasonJSON, &logsExpired, &logsRemoved, &variablesJSON)
		if err != nil {
			return nil, fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
		}

		var statusReason *models.TaskRunStatusReason = nil
		if statusReasonJSON.Valid {
			err := json.Unmarshal([]byte(statusReasonJSON.String), statusReason)
			if err != nil {
				return nil, fmt.Errorf("database error occurred; could not decode object; %v", err)
			}
		}

		var exitCode *int64 = nil
		if exitCodeRaw.Valid {
			exitCode = &exitCodeRaw.Int64
		}

		task := models.Task{}
		err = json.Unmarshal([]byte(taskJSON), &task)
		if err != nil {
			return nil, fmt.Errorf("database error occurred; could not decode object; %v", err)
		}

		variables := []models.Variable{}
		err = json.Unmarshal([]byte(variablesJSON), &variables)
		if err != nil {
			return nil, fmt.Errorf("database error occurred; could not decode object; %v", err)
		}

		taskRun.Namespace = namespace
		taskRun.Pipeline = pipeline
		taskRun.Run = run
		taskRun.ID = id
		taskRun.Task = task
		taskRun.Created = created
		taskRun.Started = started
		taskRun.Ended = ended
		taskRun.ExitCode = exitCode
		taskRun.State = models.TaskRunState(state)
		taskRun.Status = models.TaskRunStatus(status)
		taskRun.StatusReason = statusReason
		taskRun.LogsExpired = logsExpired
		taskRun.LogsRemoved = logsRemoved
		taskRun.Variables = variables

		taskRuns = append(taskRuns, taskRun)
	}
	err = rows.Err()
	if err != nil {
		return nil, fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}

	return taskRuns, nil
}

func (db *DB) InsertTaskRun(taskRun *models.TaskRun) error {
	var statusReasonJSON *string = nil
	if taskRun.StatusReason != nil {
		rawJSON, err := json.Marshal(taskRun.StatusReason)
		if err != nil {
			return fmt.Errorf("database error occurred; could not encode object; %v", err)
		}

		statusReasonJSON = ptr(string(rawJSON))
	}

	taskJSON, err := json.Marshal(taskRun.Task)
	if err != nil {
		return fmt.Errorf("database error occurred; could not encode object; %v", err)
	}

	variablesJSON, err := json.Marshal(taskRun.Variables)
	if err != nil {
		return fmt.Errorf("database error occurred; could not encode object; %v", err)
	}

	_, err = qb.Insert("task_runs").Columns("namespace", "pipeline", "run", "id", "created", "started", "ended",
		"exit_code", "logs_expired", "logs_removed", "state", "status", "status_reason", "task", "variables").Values(
		taskRun.Namespace, taskRun.Pipeline, taskRun.Run, taskRun.ID, taskRun.Created, taskRun.Started,
		taskRun.Ended, taskRun.ExitCode, taskRun.LogsExpired, taskRun.LogsRemoved, taskRun.State, taskRun.Status,
		statusReasonJSON, string(taskJSON), string(variablesJSON)).RunWith(db).Exec()
	if err != nil {
		if strings.Contains(err.Error(), "UNIQUE constraint failed") {
			return ErrEntityExists
		}

		return fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}

	return nil
}

func (db *DB) GetTaskRun(namespace, pipeline string, run int64, taskRun string) (models.TaskRun, error) {
	row := qb.Select("namespace", "pipeline", "run", "id", "task", "created", "started", "ended", "exit_code",
		"state", "status", "status_reason", "logs_expired", "logs_removed", "variables").
		From("task_runs").
		Where(qb.Eq{"namespace": namespace, "pipeline": pipeline, "run": run, "id": taskRun}).RunWith(db).QueryRow()

	var namespaceID string
	var pipelineID string
	var runID int64
	var id string
	var taskJSON string
	var created int64
	var started int64
	var ended int64
	var exitCodeRaw sql.NullInt64
	var state string
	var status string
	var statusReasonJSON sql.NullString
	var logsExpired bool
	var logsRemoved bool
	var variablesJSON string

	err := row.Scan(&namespaceID, &pipelineID, &runID, &id, &taskJSON, &created, &started, &ended,
		&exitCodeRaw, &state, &status, &statusReasonJSON, &logsExpired, &logsRemoved, &variablesJSON)
	if err != nil {
		if errors.Is(err, sql.ErrNoRows) {
			return models.TaskRun{}, ErrEntityNotFound
		}

		return models.TaskRun{}, fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}

	var statusReason *models.TaskRunStatusReason = nil
	if statusReasonJSON.Valid {
		err := json.Unmarshal([]byte(statusReasonJSON.String), statusReason)
		if err != nil {
			return models.TaskRun{}, fmt.Errorf("database error occurred; could not decode object; %v", err)
		}
	}

	task := models.Task{}
	err = json.Unmarshal([]byte(taskJSON), &task)
	if err != nil {
		return models.TaskRun{}, fmt.Errorf("database error occurred; could not decode object; %v", err)
	}

	var exitCode *int64 = nil
	if exitCodeRaw.Valid {
		exitCode = &exitCodeRaw.Int64
	}

	variables := []models.Variable{}
	err = json.Unmarshal([]byte(variablesJSON), &variables)
	if err != nil {
		return models.TaskRun{}, fmt.Errorf("database error occurred; could not decode object; %v", err)
	}

	retrievedTaskRun := models.TaskRun{}

	retrievedTaskRun.Namespace = namespace
	retrievedTaskRun.Pipeline = pipeline
	retrievedTaskRun.Run = run
	retrievedTaskRun.ID = id
	retrievedTaskRun.Task = task
	retrievedTaskRun.Created = created
	retrievedTaskRun.Started = started
	retrievedTaskRun.Ended = ended
	retrievedTaskRun.ExitCode = exitCode
	retrievedTaskRun.State = models.TaskRunState(state)
	retrievedTaskRun.Status = models.TaskRunStatus(status)
	retrievedTaskRun.StatusReason = statusReason
	retrievedTaskRun.LogsExpired = logsExpired
	retrievedTaskRun.LogsRemoved = logsRemoved
	retrievedTaskRun.Variables = variables

	return retrievedTaskRun, nil
}

func (db *DB) UpdateTaskRun(taskRun *models.TaskRun, fields UpdatableTaskRunFields) error {
	query := qb.Update("task_runs")

	if fields.Started != nil {
		query = query.Set("started", fields.Started)
	}

	if fields.Ended != nil {
		query = query.Set("ended", fields.Ended)
	}

	if fields.ExitCode != nil {
		query = query.Set("exit_code", fields.ExitCode)
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
		query = query.Set("status_reason", statusReason)
	}

	if fields.LogsExpired != nil {
		query = query.Set("logs_expired", fields.LogsExpired)
	}

	if fields.LogsRemoved != nil {
		query = query.Set("logs_removed", fields.LogsRemoved)
	}

	if fields.Variables != nil {
		variables, err := json.Marshal(fields.Variables)
		if err != nil {
			return fmt.Errorf("database error occurred; could not encode object; %v", err)
		}
		query = query.Set("variables", string(variables))
	}

	_, err := query.Where(qb.Eq{
		"namespace": taskRun.Namespace, "pipeline": taskRun.Pipeline, "run": taskRun.Run,
		"id": taskRun.ID,
	}).RunWith(db).Exec()
	if err != nil {
		if errors.Is(err, sql.ErrNoRows) {
			return ErrEntityNotFound
		}

		return fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}

	return nil
}

func (db *DB) DeleteTaskRun(namespace, pipeline string, run int64, id string) error {
	_, err := qb.Delete("task_runs").
		Where(qb.Eq{"namespace": namespace, "pipeline": pipeline, "run": run, "id": id}).RunWith(db).Exec()
	if err != nil {
		if errors.Is(err, sql.ErrNoRows) {
			return nil
		}

		return fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}

	return nil
}
