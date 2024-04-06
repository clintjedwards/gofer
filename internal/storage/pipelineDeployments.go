package storage

import (
	"database/sql"
	"errors"
	"fmt"
	"strings"

	qb "github.com/Masterminds/squirrel"
)

type PipelineDeployment struct {
	Namespace    string `db:"namespace"`
	Pipeline     string `db:"pipeline"`
	ID           int64  `db:"id"`
	StartVersion int64  `db:"start_version"`
	EndVersion   int64  `db:"end_version"`
	Started      string `db:"started"`
	Ended        string `db:"ended"`
	State        string `db:"state"`
	Status       string `db:"status"`
	StatusReason string `db:"status_reason"`
	Logs         string `db:"logs"`
}

type UpdatablePipelineDeploymentFields struct {
	Ended        *string
	State        *string
	Status       *string
	StatusReason *string
	Logs         *string
}

func (db *DB) ListPipelineDeployments(conn Queryable, offset, limit int, namespace, pipeline string) (
	[]PipelineDeployment, error,
) {
	if limit == 0 || limit > db.maxResultsLimit {
		limit = db.maxResultsLimit
	}

	query, args := qb.Select("namespace", "pipeline", "id", "start_version", "end_version", "started", "ended",
		"state", "status", "status_reason", "logs").
		From("pipeline_deployments").
		Where(qb.Eq{"namespace": namespace, "pipeline": pipeline}).
		OrderBy("id DESC").
		Limit(uint64(limit)).
		Offset(uint64(offset)).MustSql()

	deployments := []PipelineDeployment{}
	err := conn.Select(&deployments, query, args...)
	if err != nil {
		return nil, fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}

	return deployments, nil
}

func (db *DB) ListRunningPipelineDeployments(conn Queryable, offset, limit int, namespace, pipeline string) (
	[]PipelineDeployment, error,
) {
	if limit == 0 || limit > db.maxResultsLimit {
		limit = db.maxResultsLimit
	}

	query, args := qb.Select("namespace", "pipeline", "id", "start_version", "end_version", "started", "ended",
		"state", "status", "status_reason", "logs").
		From("pipeline_deployments").
		Where(qb.Eq{"namespace": namespace, "pipeline": pipeline, "state": "RUNNING"}).
		OrderBy("id DESC").
		Limit(uint64(limit)).
		Offset(uint64(offset)).MustSql()

	deployments := []PipelineDeployment{}
	err := conn.Select(&deployments, query, args...)
	if err != nil {
		return nil, fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}

	return deployments, nil
}

func (db *DB) InsertPipelineDeployment(conn Queryable, deployment *PipelineDeployment) error {
	_, err := qb.Insert("pipeline_deployments").Columns("namespace", "pipeline", "id", "start_version", "end_version",
		"started", "ended", "state", "status", "status_reason", "logs").Values(
		deployment.Namespace, deployment.Pipeline, deployment.ID, deployment.StartVersion, deployment.EndVersion,
		deployment.Started, deployment.Ended, deployment.State, deployment.Status, deployment.StatusReason,
		deployment.Logs,
	).RunWith(conn).Exec()
	if err != nil {
		if strings.Contains(err.Error(), "UNIQUE constraint failed") {
			return ErrEntityExists
		}

		return fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}

	return nil
}

func (db *DB) GetPipelineDeployment(conn Queryable, namespace, pipeline string, id int64) (
	PipelineDeployment, error,
) {
	query, args := qb.Select("namespace", "pipeline", "id", "start_version", "end_version",
		"started", "ended", "state", "status", "status_reason", "logs").
		From("pipeline_deployments").
		Where(qb.Eq{"namespace": namespace, "pipeline": pipeline, "id": id}).MustSql()

	deployment := PipelineDeployment{}
	err := conn.Get(&deployment, query, args...)
	if err != nil {
		if errors.Is(err, sql.ErrNoRows) {
			return PipelineDeployment{}, ErrEntityNotFound
		}

		return PipelineDeployment{}, fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}

	return deployment, nil
}

func (db *DB) UpdatePipelineDeployment(conn Queryable, namespace, pipeline string, id int64,
	fields UpdatablePipelineDeploymentFields,
) error {
	query := qb.Update("pipeline_deployments")

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

	if fields.Logs != nil {
		query = query.Set("logs", fields.Logs)
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

func (db *DB) DeletePipelineDeployment(conn Queryable, namespace, pipeline string, id int64) error {
	_, err := qb.Delete("pipeline_deployments").Where(qb.Eq{"namespace": namespace, "pipeline": pipeline, "id": id}).
		RunWith(conn).Exec()
	if err != nil {
		if errors.Is(err, sql.ErrNoRows) {
			return nil
		}

		return fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}

	return nil
}
