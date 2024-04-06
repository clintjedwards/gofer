package storage

import (
	"database/sql"
	"errors"
	"fmt"
	"strings"

	qb "github.com/Masterminds/squirrel"
)

type PipelineTaskExecution struct {
	Namespace    string `db:"namespace"`
	Pipeline     string `db:"pipeline"`
	Run          int64  `db:"run"`
	ID           string `db:"id"`
	TaskKind     string `db:"task_kind"`
	Task         string `db:"task"`
	Created      string `db:"created"`
	Started      string `db:"started"`
	Ended        string `db:"ended"`
	ExitCode     int64  `db:"exit_code"`
	LogsExpired  bool   `db:"logs_expired"`
	LogsRemoved  bool   `db:"logs_removed"`
	State        string `db:"state"`
	Status       string `db:"status"`
	StatusReason string `db:"status_reason"`
	Variables    string `db:"variables"`
}

type UpdatablePipelineTaskExecutionFields struct {
	Started      *string
	Ended        *string
	ExitCode     *int64
	State        *string
	Status       *string
	StatusReason *string
	LogsExpired  *bool
	LogsRemoved  *bool
	Variables    *string
}

func (db *DB) ListPipelineTaskExecutions(conn Queryable, offset, limit int, namespace, pipeline string, run int64) (
	[]PipelineTaskExecution, error,
) {
	if limit == 0 || limit > db.maxResultsLimit {
		limit = db.maxResultsLimit
	}

	query, args := qb.Select("namespace", "pipeline", "run", "id", "task_kind", "task", "created", "started", "ended", "exit_code",
		"state", "status", "status_reason", "logs_expired", "logs_removed", "variables").
		From("pipeline_task_executions").
		Where(qb.Eq{"namespace": namespace, "pipeline": pipeline, "run": run}).
		Limit(uint64(limit)).
		OrderBy("started ASC").
		Offset(uint64(offset)).MustSql()

	TaskExecutions := []PipelineTaskExecution{}
	err := conn.Select(&TaskExecutions, query, args...)
	if err != nil {
		return nil, fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}

	return TaskExecutions, nil
}

func (db *DB) InsertPipelineTaskExecution(conn Queryable, taskExecution *PipelineTaskExecution) error {
	_, err := qb.Insert("pipeline_task_executions").Columns("namespace", "pipeline", "run", "id", "created", "started", "ended",
		"exit_code", "logs_expired", "logs_removed", "state", "status", "status_reason", "task_kind", "task", "variables").Values(
		taskExecution.Namespace, taskExecution.Pipeline, taskExecution.Run, taskExecution.ID, taskExecution.Created, taskExecution.Started,
		taskExecution.Ended, taskExecution.ExitCode, taskExecution.LogsExpired, taskExecution.LogsRemoved, taskExecution.State, taskExecution.Status,
		taskExecution.StatusReason, taskExecution.TaskKind, taskExecution.Task, taskExecution.Variables).RunWith(conn).Exec()
	if err != nil {
		if strings.Contains(err.Error(), "UNIQUE constraint failed") {
			return ErrEntityExists
		}

		return fmt.Errorf("database error occurred; could not insert pipeline to DB: %v; %w", err, ErrInternal)
	}

	return nil
}

func (db *DB) GetPipelineTaskExecution(conn Queryable, namespace, pipeline string, run int64, id string) (PipelineTaskExecution, error) {
	query, args := qb.Select("namespace", "pipeline", "run", "id", "task_kind", "task", "created", "started", "ended", "exit_code",
		"state", "status", "status_reason", "logs_expired", "logs_removed", "variables").
		From("pipeline_task_executions").
		Where(qb.Eq{"namespace": namespace, "pipeline": pipeline, "run": run, "id": id}).MustSql()

	TaskExecution := PipelineTaskExecution{}
	err := conn.Get(&TaskExecution, query, args...)
	if err != nil {
		if errors.Is(err, sql.ErrNoRows) {
			return PipelineTaskExecution{}, ErrEntityNotFound
		}

		return PipelineTaskExecution{}, fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}

	return TaskExecution, nil
}

func (db *DB) UpdatePipelineTaskExecution(conn Queryable, namespace, pipeline string, run int64, id string, fields UpdatablePipelineTaskExecutionFields) error {
	query := qb.Update("pipeline_task_executions")

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
		query = query.Set("status_reason", fields.StatusReason)
	}

	if fields.LogsExpired != nil {
		query = query.Set("logs_expired", fields.LogsExpired)
	}

	if fields.LogsRemoved != nil {
		query = query.Set("logs_removed", fields.LogsRemoved)
	}

	if fields.Variables != nil {
		query = query.Set("variables", fields.Variables)
	}

	_, err := query.Where(qb.Eq{
		"namespace": namespace, "pipeline": pipeline, "run": run, "id": id,
	}).RunWith(conn).Exec()
	if err != nil {
		if errors.Is(err, sql.ErrNoRows) {
			return ErrEntityNotFound
		}

		return fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}

	return nil
}

func (db *DB) DeletePipelineTaskExecution(conn Queryable, namespace, pipeline string, run int64, id string) error {
	_, err := qb.Delete("pipeline_task_executions").
		Where(qb.Eq{"namespace": namespace, "pipeline": pipeline, "run": run, "id": id}).RunWith(conn).Exec()
	if err != nil {
		if errors.Is(err, sql.ErrNoRows) {
			return nil
		}

		return fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}

	return nil
}
