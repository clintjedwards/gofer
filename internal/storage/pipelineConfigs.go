package storage

import (
	"database/sql"
	"errors"
	"fmt"
	"strings"

	qb "github.com/Masterminds/squirrel"
)

type PipelineConfig struct {
	Namespace   string
	Pipeline    string
	Version     int64
	Parallelism int64
	Name        string
	Description string
	Registered  int64
	Deprecated  int64
	State       string
}

type UpdatablePipelineConfigFields struct {
	Deprecated *int64
	State      *string
}

func (db *DB) ListPipelineConfigs(conn Queryable, offset, limit int, namespace, pipeline string) ([]PipelineConfig, error) {
	if limit == 0 || limit > db.maxResultsLimit {
		limit = db.maxResultsLimit
	}

	query, args := qb.Select("namespace", "pipeline", "version", "parallelism", "name", "description", "registered",
		"deprecated", "state").
		From("pipeline_configs").
		OrderBy("version DESC").
		Limit(uint64(limit)).
		Offset(uint64(offset)).
		Where(qb.Eq{"namespace": namespace, "pipeline": pipeline}).
		MustSql()

	pipelineConfigs := []PipelineConfig{}
	err := conn.Select(&pipelineConfigs, query, args...)
	if err != nil {
		return nil, fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}

	return pipelineConfigs, nil
}

func (db *DB) InsertPipelineConfig(conn Queryable, config *PipelineConfig) error {
	_, err := qb.Insert("pipeline_configs").Columns("namespace", "pipeline", "version", "parallelism", "name",
		"description", "registered", "deprecated", "state").
		Values(config.Namespace, config.Pipeline, config.Version, config.Parallelism, config.Name,
			config.Description, config.Registered, config.Deprecated, config.State).
		RunWith(conn).Exec()
	if err != nil {
		if strings.Contains(err.Error(), "UNIQUE constraint failed") {
			return ErrEntityExists
		}

		return fmt.Errorf("database error occurred; could not insert pipeline to DB: %v; %w", err, ErrInternal)
	}

	return nil
}

func (db *DB) GetPipelineConfig(conn Queryable, namespace, pipeline string, version int64) (PipelineConfig, error) {
	query, args := qb.Select("namespace", "pipeline", "version", "parallelism", "name", "description", "registered",
		"deprecated", "state").
		From("pipeline_configs").Where(qb.Eq{"namespace": namespace, "pipeline": pipeline, "version": version}).
		MustSql()

	config := PipelineConfig{}
	err := conn.Get(&config, query, args...)
	if err != nil {
		if errors.Is(err, sql.ErrNoRows) {
			return PipelineConfig{}, ErrEntityNotFound
		}

		return PipelineConfig{}, fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}

	return config, nil
}

func (db *DB) GetLatestPipelineConfig(conn Queryable, namespace, pipeline string) (PipelineConfig, error) {
	query, args := qb.Select("namespace", "pipeline", "version", "parallelism", "name", "description", "registered",
		"deprecated", "state").
		From("pipeline_configs").
		OrderBy("version DESC").
		Limit(1).
		Where(qb.Eq{"namespace": namespace, "pipeline": pipeline}).
		MustSql()

	pipelineConfigs := []PipelineConfig{}
	err := conn.Select(&pipelineConfigs, query, args...)
	if err != nil {
		return PipelineConfig{}, fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}

	if len(pipelineConfigs) != 1 {
		return PipelineConfig{}, ErrEntityNotFound
	}

	return pipelineConfigs[0], nil
}

func (db *DB) GetLatestLivePipelineConfig(conn Queryable, namespace, pipeline string) (PipelineConfig, error) {
	query, args := qb.Select("namespace", "pipeline", "version", "parallelism", "name", "description", "registered",
		"deprecated", "state").
		From("pipeline_configs").
		OrderBy("version DESC").
		Limit(1).
		Where(qb.Eq{"namespace": namespace, "pipeline": pipeline, "state": "LIVE"}).
		MustSql()

	pipelineConfigs := []PipelineConfig{}
	err := conn.Select(&pipelineConfigs, query, args...)
	if err != nil {
		return PipelineConfig{}, fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}

	if len(pipelineConfigs) != 1 {
		return PipelineConfig{}, ErrEntityNotFound
	}

	return pipelineConfigs[0], nil
}

func (db *DB) UpdatePipelineConfig(conn Queryable, namespace, pipeline string, version int64,
	fields UpdatablePipelineConfigFields,
) error {
	query := qb.Update("pipeline_configs")

	if fields.Deprecated != nil {
		query = query.Set("deprecated", fields.Deprecated)
	}

	if fields.State != nil {
		query = query.Set("state", fields.State)
	}

	_, err := query.Where(qb.Eq{"namespace": namespace, "pipeline": pipeline, "version": version}).RunWith(conn).Exec()
	if err != nil {
		if errors.Is(err, sql.ErrNoRows) {
			return ErrEntityNotFound
		}

		return fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}

	return nil
}

func (db *DB) DeletePipelineConfig(conn Queryable, namespace, pipeline string, version int64) error {
	_, err := qb.Delete("pipeline_configs").
		Where(qb.Eq{"namespace": namespace, "pipeline": pipeline, "version": version}).RunWith(conn).Exec()
	if err != nil {
		if errors.Is(err, sql.ErrNoRows) {
			return nil
		}

		return fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}

	return nil
}
