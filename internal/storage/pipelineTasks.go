package storage

import (
	"database/sql"
	"errors"
	"fmt"
	"strings"

	qb "github.com/Masterminds/squirrel"
)

type PipelineTask struct {
	Namespace             string
	Pipeline              string
	PipelineConfigVersion int64 `db:"pipeline_config_version"`
	ID                    string
	Description           string
	Image                 string
	RegistryAuth          string `db:"registry_auth"`
	DependsOn             string `db:"depends_on"`
	Variables             string
	Entrypoint            string
	Command               string
	InjectAPIToken        bool `db:"inject_api_token"`
}

func (db *DB) ListPipelineTasks(conn Queryable, namespace, pipeline string, version int64) ([]PipelineTask, error) {
	query, args := qb.Select("namespace", "pipeline", "pipeline_config_version", "id", "description",
		"image", "registry_auth", "depends_on", "variables", "entrypoint", "command", "inject_api_token").
		From("pipeline_tasks").
		Where(qb.Eq{"namespace": namespace, "pipeline": pipeline, "pipeline_config_version": version}).
		MustSql()

	tasks := []PipelineTask{}
	err := conn.Select(&tasks, query, args...)
	if err != nil {
		return nil, fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}

	return tasks, nil
}

func (db *DB) InsertPipelineTask(conn Queryable, task *PipelineTask) error {
	_, err := qb.Insert("pipeline_tasks").
		Columns("namespace", "pipeline", "pipeline_config_version", "id", "description", "image",
			"registry_auth", "depends_on", "variables", "entrypoint", "command", "inject_api_token").Values(
		task.Namespace, task.Pipeline, task.PipelineConfigVersion, task.ID, task.Description,
		task.Image, task.RegistryAuth, task.DependsOn, task.Variables, task.Entrypoint, task.Command,
		task.InjectAPIToken,
	).RunWith(conn).Exec()
	if err != nil {
		if strings.Contains(err.Error(), "UNIQUE constraint failed") {
			return ErrEntityExists
		}

		return fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}

	return nil
}

func (db *DB) GetPipelineTask(conn Queryable, namespace, pipeline string, version int64, id string) (
	PipelineTask, error,
) {
	query, args := qb.Select("namespace", "pipeline", "pipeline_config_version", "id", "description", "image",
		"registry_auth", "depends_on", "variables", "entrypoint", "command", "inject_api_token").
		From("pipeline_tasks").Where(qb.Eq{
		"namespace":               namespace,
		"pipeline":                pipeline,
		"pipeline_config_version": version,
		"id":                      id,
	}).MustSql()

	task := PipelineTask{}
	err := conn.Get(&task, query, args...)
	if err != nil {
		if errors.Is(err, sql.ErrNoRows) {
			return PipelineTask{}, ErrEntityNotFound
		}

		return PipelineTask{}, fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}

	return task, nil
}
