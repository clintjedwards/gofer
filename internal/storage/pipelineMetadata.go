package storage

import (
	"database/sql"
	"errors"
	"fmt"
	"strings"

	qb "github.com/Masterminds/squirrel"
)

type PipelineMetadata struct {
	Namespace string
	ID        string
	Created   string
	Modified  string
	State     string
}

type UpdatablePipelineMetadataFields struct {
	Modified *string
	State    *string
}

// Returns the total number of pipelines over all namespaces.
func (db *DB) GetPipelineCount(conn Queryable) (int64, error) {
	query, args := qb.Select("COUNT(*)").
		From("pipeline_metadata").
		MustSql()

	var count int64
	err := conn.QueryRow(query, args...).Scan(&count)
	if err != nil {
		return 0, fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}

	return count, nil
}

// Returns pipelines ordered by id.
func (db *DB) ListPipelineMetadata(conn Queryable, offset, limit int, namespace string) ([]PipelineMetadata, error) {
	if limit == 0 || limit > db.maxResultsLimit {
		limit = db.maxResultsLimit
	}

	query, args := qb.Select("namespace", "id", "created", "modified", "state").
		From("pipeline_metadata").
		OrderBy("id DESC").
		Limit(uint64(limit)).
		Offset(uint64(offset)).
		Where(qb.Eq{"namespace": namespace}).
		MustSql()

	pipelines := []PipelineMetadata{}
	err := conn.Select(&pipelines, query, args...)
	if err != nil {
		return nil, fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}

	return pipelines, nil
}

func (db *DB) InsertPipelineMetadata(conn Queryable, metadata *PipelineMetadata) error {
	_, err := qb.Insert("pipeline_metadata").Columns("namespace", "id", "created", "modified", "state").
		Values(metadata.Namespace, metadata.ID, metadata.Created, metadata.Modified, metadata.State).
		RunWith(conn).Exec()
	if err != nil {
		if strings.Contains(err.Error(), "UNIQUE constraint failed") {
			return ErrEntityExists
		}

		return fmt.Errorf("database error occurred; could not insert pipeline to DB: %v; %w", err, ErrInternal)
	}

	return nil
}

func (db *DB) GetPipelineMetadata(conn Queryable, namespace, id string) (PipelineMetadata, error) {
	query, args := qb.Select("namespace", "id", "created", "modified", "state").
		From("pipeline_metadata").Where(qb.Eq{"namespace": namespace, "id": id}).MustSql()

	metadata := PipelineMetadata{}
	err := conn.Get(&metadata, query, args...)
	if err != nil {
		if errors.Is(err, sql.ErrNoRows) {
			return PipelineMetadata{}, ErrEntityNotFound
		}

		return PipelineMetadata{}, fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}

	return metadata, nil
}

func (db *DB) UpdatePipelineMetadata(conn Queryable, namespace, id string, fields UpdatablePipelineMetadataFields) error {
	query := qb.Update("pipeline_metadata")

	if fields.Modified != nil {
		query = query.Set("modified", fields.Modified)
	}

	if fields.State != nil {
		query = query.Set("state", fields.State)
	}

	_, err := query.Where(qb.Eq{"namespace": namespace, "id": id}).RunWith(conn).Exec()
	if err != nil {
		if errors.Is(err, sql.ErrNoRows) {
			return ErrEntityNotFound
		}

		return fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}

	return nil
}

func (db *DB) DeletePipelineMetadata(conn Queryable, namespace, id string) error {
	_, err := qb.Delete("pipeline_metadata").Where(qb.Eq{"namespace": namespace, "id": id}).RunWith(conn).Exec()
	if err != nil {
		if errors.Is(err, sql.ErrNoRows) {
			return nil
		}

		return fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}

	return nil
}
