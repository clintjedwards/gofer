package storage

import (
	"database/sql"
	"errors"
	"fmt"
	"strings"

	qb "github.com/Masterminds/squirrel"
)

type PipelineRun struct {
	Namespace             string
	Pipeline              string
	PipelineConfigVersion int64 `db:"pipeline_config_version"`
	ID                    int64
	Started               int64
	Ended                 int64
	State                 string
	Status                string
	StatusReason          string `db:"status_reason"`
	Initiator             string
	Variables             string
	StoreObjectsExpired   bool `db:"store_objects_expired"`
}

type UpdatablePipelineRunFields struct {
	Ended               *int64
	State               *string
	Status              *string
	StatusReason        *string
	Variables           *string
	StoreObjectsExpired *bool
}

func (db *DB) ListPipelineRuns(conn Queryable, offset, limit int, namespace, pipeline string) ([]PipelineRun, error) {
	if limit == 0 || limit > db.maxResultsLimit {
		limit = db.maxResultsLimit
	}

	query, args := qb.Select("namespace", "pipeline", "pipeline_config_version", "id", "started", "ended", "state",
		"status", "status_reason", "initiator", "variables", "store_objects_expired").
		From("pipeline_runs").
		Where(qb.Eq{"namespace": namespace, "pipeline": pipeline}).
		OrderBy("id DESC").
		Limit(uint64(limit)).
		Offset(uint64(offset)).MustSql()

	runs := []PipelineRun{}
	err := conn.Select(&runs, query, args...)
	if err != nil {
		return nil, fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}

	return runs, nil
}

func (db *DB) InsertPipelineRun(conn Queryable, run *PipelineRun) error {
	_, err := qb.Insert("pipeline_runs").Columns("namespace", "pipeline", "pipeline_config_version", "id", "started",
		"ended", "state", "status", "status_reason", "initiator", "variables", "store_objects_expired").Values(
		run.Namespace, run.Pipeline, run.PipelineConfigVersion, run.ID, run.Started, run.Ended, run.State,
		run.Status, run.StatusReason, run.Initiator, run.Variables, run.StoreObjectsExpired,
	).RunWith(conn).Exec()
	if err != nil {
		if strings.Contains(err.Error(), "UNIQUE constraint failed") {
			return ErrEntityExists
		}

		return fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}

	return nil
}

func (db *DB) GetPipelineRun(conn Queryable, namespace, pipeline string, id int64) (PipelineRun, error) {
	query, args := qb.Select("namespace", "pipeline", "pipeline_config_version", "id", "started", "ended", "state", "status",
		"status_reason", "initiator", "variables", "store_objects_expired").
		From("pipeline_runs").
		Where(qb.Eq{"namespace": namespace, "pipeline": pipeline, "id": id}).MustSql()

	run := PipelineRun{}
	err := conn.Get(&run, query, args...)
	if err != nil {
		if errors.Is(err, sql.ErrNoRows) {
			return PipelineRun{}, ErrEntityNotFound
		}

		return PipelineRun{}, fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}

	return run, nil
}

func (db *DB) GetLatestPipelineRun(conn Queryable, namespace, pipeline string) (PipelineRun, error) {
	query, args := qb.Select("namespace", "pipeline", "pipeline_config_version", "id", "started", "ended", "state",
		"status", "status_reason", "initiator", "variables", "store_objects_expired").
		From("pipeline_runs").
		Where(qb.Eq{"namespace": namespace, "pipeline": pipeline}).
		OrderBy("id DESC").
		Limit(1).MustSql()

	runs := []PipelineRun{}
	err := conn.Select(&runs, query, args...)
	if err != nil {
		return PipelineRun{}, fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}

	if len(runs) < 1 {
		return PipelineRun{}, ErrEntityNotFound
	}

	return runs[0], nil
}

func (db *DB) UpdatePipelineRun(conn Queryable, namespace, pipeline string, id int64, fields UpdatablePipelineRunFields) error {
	query := qb.Update("pipeline_runs")

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
		query = query.Set("status_reason", fields.StatusReason)
	}

	if fields.Variables != nil {
		query = query.Set("variables", fields.Variables)
	}

	if fields.StoreObjectsExpired != nil {
		query = query.Set("store_objects_expired", fields.StoreObjectsExpired)
	}

	_, err := query.Where(qb.Eq{"namespace": namespace, "pipeline": pipeline, "id": id}).RunWith(conn).Exec()
	if err != nil {
		if errors.Is(err, sql.ErrNoRows) {
			return ErrEntityNotFound
		}

		return fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}

	return nil
}

func (db *DB) DeletePipelineRun(conn Queryable, namespace, pipeline string, id int64) error {
	_, err := qb.Delete("pipeline_runs").Where(qb.Eq{"namespace": namespace, "pipeline": pipeline, "id": id}).
		RunWith(conn).Exec()
	if err != nil {
		if errors.Is(err, sql.ErrNoRows) {
			return nil
		}

		return fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}

	return nil
}
