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

type UpdatablePipelineFields struct {
	Name        *string
	Description *string
	Parallelism *int64
	Modified    *int64
	State       *models.PipelineState
	Tasks       *map[string]models.Task
	Triggers    *map[string]models.PipelineTriggerSettings
	CommonTasks *map[string]models.PipelineCommonTaskSettings
	Errors      *[]models.PipelineError
}

func (db *DB) ListTasks(conn qb.BaseRunner, namespace, pipeline string) ([]models.Task, error) {
	if conn == nil {
		conn = db
	}

	rows, err := qb.Select("id", "description", "image", "registry_auth", "depends_on", "variables", "entrypoint", "command").
		From("tasks").
		Where(qb.Eq{"namespace": namespace, "pipeline": pipeline}).RunWith(conn).Query()
	if err != nil {
		return nil, fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}
	err = rows.Err()
	if err != nil {
		return nil, fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}
	defer rows.Close()

	tasks := []models.Task{}

	for rows.Next() {
		var id string
		var description string
		var image string
		var registryAuthJSON sql.NullString
		var dependsOnJSON string
		var variablesJSON string
		var entrypointJSON string
		var commandJSON string

		err = rows.Scan(&id, &description, &image, &registryAuthJSON, &dependsOnJSON, &variablesJSON, &entrypointJSON, &commandJSON)
		if err != nil {
			return nil, fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
		}

		var registryAuth *models.RegistryAuth = nil
		if registryAuthJSON.Valid {
			err := json.Unmarshal([]byte(registryAuthJSON.String), registryAuth)
			if err != nil {
				return nil, fmt.Errorf("database error occurred; could not decode object; %v", err)
			}
		}

		dependsOn := map[string]models.RequiredParentStatus{}
		err = json.Unmarshal([]byte(dependsOnJSON), &dependsOn)
		if err != nil {
			return nil, fmt.Errorf("database error occurred; could not decode object; %v", err)
		}

		variables := []models.Variable{}
		err = json.Unmarshal([]byte(variablesJSON), &variables)
		if err != nil {
			return nil, fmt.Errorf("database error occurred; could not decode object; %v", err)
		}

		entrypoint := []string{}
		err = json.Unmarshal([]byte(entrypointJSON), &entrypoint)
		if err != nil {
			return nil, fmt.Errorf("database error occurred; could not decode object; %v", err)
		}

		command := []string{}
		err = json.Unmarshal([]byte(commandJSON), &command)
		if err != nil {
			return nil, fmt.Errorf("database error occurred; could not decode object; %v", err)
		}

		tasks = append(tasks, models.Task{
			ID:           id,
			Description:  description,
			Image:        image,
			RegistryAuth: registryAuth,
			DependsOn:    dependsOn,
			Variables:    variables,
			Entrypoint:   entrypoint,
			Command:      command,
		})
	}
	err = rows.Err()
	if err != nil {
		return nil, fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}

	return tasks, nil
}

func (db *DB) ListTriggerSettings(conn qb.BaseRunner, namespace, pipeline string) ([]models.PipelineTriggerSettings, error) {
	if conn == nil {
		conn = db
	}

	rows, err := qb.Select("name", "label", "settings").From("pipeline_trigger_settings").
		Where(qb.Eq{"namespace": namespace, "pipeline": pipeline}).RunWith(conn).Query()
	if err != nil {
		return nil, fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}
	err = rows.Err()
	if err != nil {
		return nil, fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}
	defer rows.Close()

	triggers := []models.PipelineTriggerSettings{}

	for rows.Next() {
		var name string
		var label string
		var settingsJSON string

		err = rows.Scan(&name, &label, &settingsJSON)
		if err != nil {
			return nil, fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
		}

		settings := map[string]string{}
		err = json.Unmarshal([]byte(settingsJSON), &settings)
		if err != nil {
			return nil, fmt.Errorf("database error occurred; could not decode object; %v", err)
		}

		triggers = append(triggers, models.PipelineTriggerSettings{
			Name:     name,
			Label:    label,
			Settings: settings,
		})
	}
	err = rows.Err()
	if err != nil {
		return nil, fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}

	return triggers, nil
}

func (db *DB) ListCommonTaskSettings(conn qb.BaseRunner, namespace, pipeline string) ([]models.PipelineCommonTaskSettings, error) {
	if conn == nil {
		conn = db
	}

	rows, err := qb.Select("name", "label", "settings").From("pipeline_common_task_settings").
		Where(qb.Eq{"namespace": namespace, "pipeline": pipeline}).RunWith(conn).Query()
	if err != nil {
		return nil, fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}
	err = rows.Err()
	if err != nil {
		return nil, fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}
	defer rows.Close()

	commonTasks := []models.PipelineCommonTaskSettings{}

	for rows.Next() {
		var name string
		var label string
		var settingsJSON string

		err = rows.Scan(&name, &label, &settingsJSON)
		if err != nil {
			return nil, fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
		}

		settings := map[string]string{}
		err = json.Unmarshal([]byte(settingsJSON), &settings)
		if err != nil {
			return nil, fmt.Errorf("database error occurred; could not decode object; %v", err)
		}

		commonTasks = append(commonTasks, models.PipelineCommonTaskSettings{
			Name:     name,
			Label:    label,
			Settings: settings,
		})
	}
	err = rows.Err()
	if err != nil {
		return nil, fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}

	return commonTasks, nil
}

func (db *DB) ListPipelines(offset, limit int, namespace string) ([]models.Pipeline, error) {
	if limit == 0 || limit > db.maxResultsLimit {
		limit = db.maxResultsLimit
	}

	rows, err := qb.Select("namespace", "id", "name", "description", "parallelism", "created", "modified", "state", "errors").
		From("pipelines").
		OrderBy("created").
		Limit(uint64(limit)).
		Offset(uint64(offset)).
		Where(qb.Eq{"namespace": namespace}).RunWith(db).Query()
	if err != nil {
		return nil, fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}
	defer rows.Close()

	pipelines := []models.Pipeline{}

	for rows.Next() {
		pipeline := models.Pipeline{}

		var namespace string
		var id string
		var name string
		var description string
		var parallelism int64
		var created int64
		var modified int64
		var state string
		var errorsJSON string

		err = rows.Scan(&namespace, &id, &name, &description, &parallelism, &created, &modified, &state, &errorsJSON)
		if err != nil {
			return nil, fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
		}

		errors := []models.PipelineError{}
		err := json.Unmarshal([]byte(errorsJSON), &errors)
		if err != nil {
			return nil, fmt.Errorf("database error occurred; could not decode object; %v", err)
		}

		pipeline.Namespace = namespace
		pipeline.ID = id
		pipeline.Name = name
		pipeline.Description = description
		pipeline.Parallelism = parallelism
		pipeline.Created = created
		pipeline.Modified = modified
		pipeline.State = models.PipelineState(state)
		pipeline.Errors = errors

		pipelines = append(pipelines, pipeline)
	}
	err = rows.Err()
	if err != nil {
		return nil, fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}

	for index, pipeline := range pipelines {
		taskList, err := db.ListTasks(nil, namespace, pipeline.ID)
		if err != nil {
			return nil, fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
		}

		tasks := map[string]models.Task{}
		for _, task := range taskList {
			tasks[task.ID] = task
		}

		triggerList, err := db.ListTriggerSettings(nil, namespace, pipeline.ID)
		if err != nil {
			return nil, fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
		}

		triggers := map[string]models.PipelineTriggerSettings{}
		for _, trigger := range triggerList {
			triggers[trigger.Label] = trigger
		}

		commonTaskList, err := db.ListCommonTaskSettings(nil, namespace, pipeline.ID)
		if err != nil {
			return nil, fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
		}

		commonTasks := map[string]models.PipelineCommonTaskSettings{}
		for _, task := range commonTaskList {
			commonTasks[task.Label] = task
		}

		pipeline.Tasks = tasks
		pipeline.Triggers = triggers
		pipeline.CommonTasks = commonTasks

		pipelines[index] = pipeline
	}

	return pipelines, nil
}

func insertTask(conn qb.BaseRunner, namespace, pipeline string, task *models.Task) error {
	dependsOnJSON, err := json.Marshal(task.DependsOn)
	if err != nil {
		return fmt.Errorf("database error occurred; could not encode object; %v", err)
	}

	variablesJSON, err := json.Marshal(task.Variables)
	if err != nil {
		return fmt.Errorf("database error occurred; could not encode object; %v", err)
	}

	var registryAuthJSON *string = nil
	if task.RegistryAuth != nil {
		tmpJSON, err := json.Marshal(task.RegistryAuth)
		if err != nil {
			return fmt.Errorf("database error occurred; could not encode object; %v", err)
		}

		registryAuthJSON = ptr(string(tmpJSON))
	}

	entrypointJSON, err := json.Marshal(task.Entrypoint)
	if err != nil {
		return fmt.Errorf("database error occurred; could not encode object; %v", err)
	}

	commandJSON, err := json.Marshal(task.Command)
	if err != nil {
		return fmt.Errorf("database error occurred; could not encode object; %v", err)
	}

	_, err = qb.Insert("tasks").Columns("namespace", "pipeline", "id", "description", "image",
		"registry_auth", "depends_on", "variables", "entrypoint", "command").Values(
		namespace, pipeline, task.ID, task.Description, task.Image, registryAuthJSON,
		string(dependsOnJSON), string(variablesJSON), string(entrypointJSON), string(commandJSON),
	).RunWith(conn).Exec()
	if err != nil {
		if strings.Contains(err.Error(), "UNIQUE constraint failed") {
			return ErrEntityExists
		}

		return fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}

	return nil
}

func insertTriggerSettings(conn qb.BaseRunner, namespace, pipeline string, settings *models.PipelineTriggerSettings) error {
	settingsJSON, err := json.Marshal(settings.Settings)
	if err != nil {
		return fmt.Errorf("database error occurred; could not encode object; %v", err)
	}

	_, err = qb.Insert("pipeline_trigger_settings").Columns("namespace", "pipeline", "name", "label", "settings").Values(
		namespace, pipeline, settings.Name, settings.Label, string(settingsJSON)).RunWith(conn).Exec()
	if err != nil {
		if strings.Contains(err.Error(), "UNIQUE constraint failed") {
			return ErrEntityExists
		}

		return fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}

	return nil
}

func insertCommonTaskSettings(conn qb.BaseRunner, namespace, pipeline string, settings *models.PipelineCommonTaskSettings) error {
	settingsJSON, err := json.Marshal(settings.Settings)
	if err != nil {
		return fmt.Errorf("database error occurred; could not encode object; %v", err)
	}

	_, err = qb.Insert("pipeline_common_task_settings").Columns("namespace", "pipeline", "name", "label", "settings").Values(
		namespace, pipeline, settings.Name, settings.Label, string(settingsJSON)).RunWith(conn).Exec()
	if err != nil {
		if strings.Contains(err.Error(), "UNIQUE constraint failed") {
			return ErrEntityExists
		}

		return fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}

	return nil
}

func (db *DB) InsertPipeline(pipeline *models.Pipeline) error {
	tx, err := db.Begin()
	if err != nil {
		mustRollback(tx)
		return fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}

	errorsJSON, err := json.Marshal(pipeline.Errors)
	if err != nil {
		mustRollback(tx)
		return fmt.Errorf("database error occurred; could not encode object; %v", err)
	}

	_, err = qb.Insert("pipelines").Columns("namespace", "id", "name", "description", "parallelism", "state",
		"created", "modified", "errors").Values(
		pipeline.Namespace, pipeline.ID, pipeline.Name, pipeline.Description, pipeline.Parallelism,
		pipeline.State, pipeline.Created, pipeline.Modified, string(errorsJSON)).RunWith(tx).Exec()
	if err != nil {
		mustRollback(tx)
		if strings.Contains(err.Error(), "UNIQUE constraint failed") {
			return ErrEntityExists
		}

		return fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}

	for _, task := range pipeline.Tasks {
		err = insertTask(tx, pipeline.Namespace, pipeline.ID, &task)
		if err != nil {
			mustRollback(tx)
			if strings.Contains(err.Error(), "UNIQUE constraint failed") {
				return ErrEntityExists
			}

			return fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
		}
	}

	for _, settings := range pipeline.Triggers {
		err = insertTriggerSettings(tx, pipeline.Namespace, pipeline.ID, &settings)
		if err != nil {
			mustRollback(tx)
			if strings.Contains(err.Error(), "UNIQUE constraint failed") {
				return ErrEntityExists
			}

			return fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
		}
	}

	for _, settings := range pipeline.CommonTasks {
		err = insertCommonTaskSettings(tx, pipeline.Namespace, pipeline.ID, &settings)
		if err != nil {
			mustRollback(tx)
			if strings.Contains(err.Error(), "UNIQUE constraint failed") {
				return ErrEntityExists
			}

			return fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
		}
	}

	err = tx.Commit()
	if err != nil {
		mustRollback(tx)
		return fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}

	return nil
}

func (db *DB) GetPipeline(conn qb.BaseRunner, namespace, pipeline string) (models.Pipeline, error) {
	if conn == nil {
		conn = db
	}

	row := qb.Select("id", "name", "description", "parallelism", "created", "modified", "state", "errors").
		From("pipelines").Where(qb.Eq{"namespace": namespace, "id": pipeline}).RunWith(conn).QueryRow()

	var id string
	var name string
	var description string
	var parallelism int64
	var created int64
	var modified int64
	var state string
	var errorsJSON string
	err := row.Scan(&id, &name, &description, &parallelism, &created, &modified, &state, &errorsJSON)
	if err != nil {
		if errors.Is(err, sql.ErrNoRows) {
			return models.Pipeline{}, ErrEntityNotFound
		}

		return models.Pipeline{}, fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}

	retrievedPipeline := models.Pipeline{}

	errors := []models.PipelineError{}
	err = json.Unmarshal([]byte(errorsJSON), &errors)
	if err != nil {
		return models.Pipeline{}, fmt.Errorf("database error occurred; could not decode object; %v", err)
	}

	taskList, err := db.ListTasks(conn, namespace, pipeline)
	if err != nil {
		return models.Pipeline{}, fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}

	tasks := map[string]models.Task{}
	for _, task := range taskList {
		tasks[task.ID] = task
	}

	triggerList, err := db.ListTriggerSettings(conn, namespace, pipeline)
	if err != nil {
		return models.Pipeline{}, fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}

	triggers := map[string]models.PipelineTriggerSettings{}
	for _, trigger := range triggerList {
		triggers[trigger.Label] = trigger
	}

	commonTaskList, err := db.ListCommonTaskSettings(conn, namespace, pipeline)
	if err != nil {
		return models.Pipeline{}, fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}

	commonTasks := map[string]models.PipelineCommonTaskSettings{}
	for _, task := range commonTaskList {
		commonTasks[task.Label] = task
	}

	retrievedPipeline.Namespace = namespace
	retrievedPipeline.ID = id
	retrievedPipeline.Name = name
	retrievedPipeline.Description = description
	retrievedPipeline.Parallelism = parallelism
	retrievedPipeline.Created = created
	retrievedPipeline.Modified = modified
	retrievedPipeline.State = models.PipelineState(state)
	retrievedPipeline.Tasks = tasks
	retrievedPipeline.Triggers = triggers
	retrievedPipeline.CommonTasks = commonTasks
	retrievedPipeline.Errors = errors

	return retrievedPipeline, nil
}

func (db *DB) UpdatePipeline(namespace, id string, fields UpdatablePipelineFields) error {
	tx, err := db.Begin()
	if err != nil {
		mustRollback(tx)
		return fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}

	pipeline, err := db.GetPipeline(tx, namespace, id)
	if err != nil {
		if errors.Is(err, sql.ErrNoRows) {
			return ErrEntityNotFound
		}

		return fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}

	query := qb.Update("pipelines")

	if fields.Name != nil {
		query = query.Set("name", fields.Name)
	}

	if fields.Description != nil {
		query = query.Set("description", fields.Description)
	}

	if fields.Parallelism != nil {
		query = query.Set("parallelism", fields.Parallelism)
	}

	if fields.State != nil {
		query = query.Set("state", fields.State)
	}

	if fields.Modified != nil {
		query = query.Set("modified", fields.Modified)
	}

	if fields.Errors != nil {
		errorsJSON, err := json.Marshal(fields.Errors)
		if err != nil {
			mustRollback(tx)
			return fmt.Errorf("database error occurred; could not encode object; %v", err)
		}
		query = query.Set("errors", errorsJSON)
	}

	if fields.Tasks != nil {
		for id := range pipeline.Tasks {
			err := deleteTask(tx, namespace, pipeline.ID, id)
			if err != nil {
				mustRollback(tx)
				return fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
			}
		}

		for _, task := range *fields.Tasks {
			err := insertTask(tx, namespace, pipeline.ID, &task)
			if err != nil {
				mustRollback(tx)
				return fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
			}
		}
	}

	if fields.Triggers != nil {
		for id := range pipeline.Triggers {
			err := deleteTriggerSettings(tx, namespace, pipeline.ID, id)
			if err != nil {
				mustRollback(tx)
				return fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
			}
		}

		for _, trigger := range *fields.Triggers {
			err := insertTriggerSettings(tx, namespace, pipeline.ID, &trigger)
			if err != nil {
				mustRollback(tx)
				return fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
			}
		}
	}

	if fields.CommonTasks != nil {
		for id := range pipeline.CommonTasks {
			err := deleteCommonTaskSettings(tx, namespace, pipeline.ID, id)
			if err != nil {
				mustRollback(tx)
				return fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
			}
		}

		for _, commonTask := range *fields.CommonTasks {
			err := insertCommonTaskSettings(tx, namespace, pipeline.ID, &commonTask)
			if err != nil {
				mustRollback(tx)
				return fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
			}
		}
	}

	_, err = query.Where(qb.Eq{"namespace": namespace, "id": id}).RunWith(tx).Exec()
	if err != nil {
		if errors.Is(err, sql.ErrNoRows) {
			return ErrEntityNotFound
		}

		return fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}

	err = tx.Commit()
	if err != nil {
		mustRollback(tx)
		return fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}

	return nil
}

func deleteTask(conn qb.BaseRunner, namespace, pipeline, id string) error {
	_, err := qb.Delete("tasks").Where(qb.Eq{"namespace": namespace, "pipeline": pipeline, "id": id}).RunWith(conn).Exec()
	if err != nil {
		if errors.Is(err, sql.ErrNoRows) {
			return nil
		}

		return fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}

	return nil
}

func deleteTriggerSettings(conn qb.BaseRunner, namespace, pipeline, label string) error {
	_, err := qb.Delete("pipeline_trigger_settings").
		Where(qb.Eq{"namespace": namespace, "pipeline": pipeline, "label": label}).RunWith(conn).Exec()
	if err != nil {
		if errors.Is(err, sql.ErrNoRows) {
			return nil
		}

		return fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}

	return nil
}

func deleteCommonTaskSettings(conn qb.BaseRunner, namespace, pipeline, label string) error {
	_, err := qb.Delete("pipeline_common_task_settings").
		Where(qb.Eq{"namespace": namespace, "pipeline": pipeline, "label": label}).RunWith(conn).Exec()
	if err != nil {
		if errors.Is(err, sql.ErrNoRows) {
			return nil
		}

		return fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}

	return nil
}

func (db *DB) DeletePipeline(namespace, id string) error {
	_, err := qb.Delete("pipelines").Where(qb.Eq{"namespace": namespace, "id": id}).RunWith(db).Exec()
	if err != nil {
		if errors.Is(err, sql.ErrNoRows) {
			return nil
		}

		return fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}

	return nil
}
