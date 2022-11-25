package storage

import (
	"database/sql"
	"errors"
	"fmt"
	"strings"

	qb "github.com/Masterminds/squirrel"
)

type PipelineTaskRun struct {
	Namespace    string
	Pipeline     string
	Run          int64
	ID           string
	TaskKind     string `db:"task_kind"`
	Task         string
	Created      int64
	Started      int64
	Ended        int64
	ExitCode     int64 `db:"exit_code"`
	LogsExpired  bool  `db:"logs_expired"`
	LogsRemoved  bool  `db:"logs_removed"`
	State        string
	Status       string
	StatusReason string `db:"status_reason"`
	Variables    string
}

type UpdatablePipelineTaskRunFields struct {
	Started      *int64
	Ended        *int64
	ExitCode     *int64
	State        *string
	Status       *string
	StatusReason *string
	LogsExpired  *bool
	LogsRemoved  *bool
	Variables    *string
}

func (db *DB) ListPipelineTaskRuns(conn Queryable, offset, limit int, namespace, pipeline string, run int64) (
	[]PipelineTaskRun, error,
) {
	if limit == 0 || limit > db.maxResultsLimit {
		limit = db.maxResultsLimit
	}

	query, args := qb.Select("namespace", "pipeline", "run", "id", "task_kind", "task", "created", "started", "ended", "exit_code",
		"state", "status", "status_reason", "logs_expired", "logs_removed", "variables").
		From("pipeline_task_runs").
		Where(qb.Eq{"namespace": namespace, "pipeline": pipeline, "run": run}).
		Limit(uint64(limit)).
		OrderBy("started ASC").
		Offset(uint64(offset)).MustSql()

	taskRuns := []PipelineTaskRun{}
	err := conn.Select(&taskRuns, query, args...)
	if err != nil {
		return nil, fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}

	return taskRuns, nil
}

func (db *DB) InsertPipelineTaskRun(conn Queryable, taskRun *PipelineTaskRun) error {
	_, err := qb.Insert("pipeline_task_runs").Columns("namespace", "pipeline", "run", "id", "created", "started", "ended",
		"exit_code", "logs_expired", "logs_removed", "state", "status", "status_reason", "task_kind", "task", "variables").Values(
		taskRun.Namespace, taskRun.Pipeline, taskRun.Run, taskRun.ID, taskRun.Created, taskRun.Started,
		taskRun.Ended, taskRun.ExitCode, taskRun.LogsExpired, taskRun.LogsRemoved, taskRun.State, taskRun.Status,
		taskRun.StatusReason, taskRun.TaskKind, taskRun.Task, taskRun.Variables).RunWith(conn).Exec()
	if err != nil {
		if strings.Contains(err.Error(), "UNIQUE constraint failed") {
			return ErrEntityExists
		}

		return fmt.Errorf("database error occurred; could not insert pipeline to DB: %v; %w", err, ErrInternal)
	}

	return nil
}

func (db *DB) GetPipelineTaskRun(conn Queryable, namespace, pipeline string, run int64, id string) (PipelineTaskRun, error) {
	query, args := qb.Select("namespace", "pipeline", "run", "id", "task_kind", "task", "created", "started", "ended", "exit_code",
		"state", "status", "status_reason", "logs_expired", "logs_removed", "variables").
		From("pipeline_task_runs").
		Where(qb.Eq{"namespace": namespace, "pipeline": pipeline, "run": run, "id": id}).MustSql()

	taskRun := PipelineTaskRun{}
	err := conn.Get(&taskRun, query, args...)
	if err != nil {
		if errors.Is(err, sql.ErrNoRows) {
			return PipelineTaskRun{}, ErrEntityNotFound
		}

		return PipelineTaskRun{}, fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}

	return taskRun, nil
}

func (db *DB) UpdatePipelineTaskRun(conn Queryable, namespace, pipeline string, run int64, id string, fields UpdatablePipelineTaskRunFields) error {
	query := qb.Update("pipeline_task_runs")

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

func (db *DB) DeletePipelineTaskRun(conn Queryable, namespace, pipeline string, run int64, id string) error {
	_, err := qb.Delete("pipeline_task_runs").
		Where(qb.Eq{"namespace": namespace, "pipeline": pipeline, "run": run, "id": id}).RunWith(conn).Exec()
	if err != nil {
		if errors.Is(err, sql.ErrNoRows) {
			return nil
		}

		return fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}

	return nil
}
