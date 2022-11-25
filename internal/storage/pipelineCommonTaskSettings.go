package storage

import (
	"database/sql"
	"errors"
	"fmt"
	"strings"

	qb "github.com/Masterminds/squirrel"
)

type PipelineCommonTaskSettings struct {
	Namespace             string
	Pipeline              string
	PipelineConfigVersion int64 `db:"pipeline_config_version"`
	Name                  string
	Label                 string
	Description           string
	DependsOn             string `db:"depends_on"`
	Settings              string
	InjectAPIToken        bool `db:"inject_api_token"`
}

func (db *DB) ListPipelineCommonTaskSettings(conn Queryable, namespace, pipeline string, version int64) (
	[]PipelineCommonTaskSettings, error,
) {
	query, args := qb.Select("namespace", "pipeline", "pipeline_config_version", "name", "label", "description",
		"depends_on", "settings", "inject_api_token").
		From("pipeline_common_task_settings").
		Where(qb.Eq{"namespace": namespace, "pipeline": pipeline, "pipeline_config_version": version}).
		MustSql()

	commonTasks := []PipelineCommonTaskSettings{}
	err := conn.Select(&commonTasks, query, args...)
	if err != nil {
		return nil, fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}

	return commonTasks, nil
}

func (db *DB) InsertPipelineCommonTaskSettings(conn Queryable, settings *PipelineCommonTaskSettings) error {
	_, err := qb.Insert("pipeline_common_task_settings").
		Columns("namespace", "pipeline", "pipeline_config_version", "name", "label", "description",
			"depends_on", "settings", "inject_api_token").Values(
		settings.Namespace, settings.Pipeline, settings.PipelineConfigVersion, settings.Name,
		settings.Label, settings.Description, settings.DependsOn, settings.Settings, settings.InjectAPIToken).
		RunWith(conn).Exec()
	if err != nil {
		if strings.Contains(err.Error(), "UNIQUE constraint failed") {
			return ErrEntityExists
		}

		return fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}

	return nil
}

func (db *DB) GetPipelineCommonTaskSettings(conn Queryable, namespace, pipeline string, version int64, label string) (
	PipelineCommonTaskSettings, error,
) {
	query, args := qb.Select("namespace", "pipeline", "pipeline_config_version", "name", "label", "description",
		"depends_on", "settings", "inject_api_token").
		From("pipeline_common_task_settings").Where(qb.Eq{
		"namespace":               namespace,
		"pipeline":                pipeline,
		"pipeline_config_version": version,
		"label":                   label,
	}).MustSql()

	settings := PipelineCommonTaskSettings{}
	err := conn.Get(&settings, query, args...)
	if err != nil {
		if errors.Is(err, sql.ErrNoRows) {
			return PipelineCommonTaskSettings{}, ErrEntityNotFound
		}

		return PipelineCommonTaskSettings{}, fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}

	return settings, nil
}
