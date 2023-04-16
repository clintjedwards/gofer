package storage

import (
	"database/sql"
	"errors"
	"fmt"
	"strings"

	qb "github.com/Masterminds/squirrel"
)

type PipelineExtensionSubscription struct {
	Namespace    string
	Pipeline     string
	Name         string
	Label        string
	Settings     string
	Status       string
	StatusReason string `db:"status_reason"`
}

type UpdateablePipelineExtensionSubscriptionFields struct {
	Settings     *string
	Status       *string
	StatusReason *string
}

func (db *DB) ListPipelineExtensionSubscriptions(conn Queryable, namespace, pipeline string) ([]PipelineExtensionSubscription, error) {
	query, args := qb.Select("namespace", "pipeline", "name", "label", "settings", "status", "status_reason").
		From("pipeline_extension_subscriptions").
		Where(qb.Eq{"namespace": namespace, "pipeline": pipeline}).
		MustSql()

	extensions := []PipelineExtensionSubscription{}
	err := conn.Select(&extensions, query, args...)
	if err != nil {
		return nil, fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}

	return extensions, nil
}

func (db *DB) InsertPipelineExtensionSubscription(conn Queryable, sub *PipelineExtensionSubscription) error {
	_, err := qb.Insert("pipeline_extension_subscriptions").
		Columns("namespace", "pipeline", "name", "label", "settings", "status", "status_reason").Values(
		sub.Namespace, sub.Pipeline, sub.Name, sub.Label, sub.Settings,
		sub.Status, sub.StatusReason,
	).RunWith(conn).Exec()
	if err != nil {
		if strings.Contains(err.Error(), "UNIQUE constraint failed") {
			return ErrEntityExists
		}

		return fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}

	return nil
}

func (db *DB) GetPipelineExtensionSubscription(conn Queryable, namespace, pipeline, name, label string) (
	PipelineExtensionSubscription, error,
) {
	query, args := qb.Select("namespace", "pipeline", "name", "label", "settings", "status", "status_reason").
		From("pipeline_extension_subscriptions").Where(qb.Eq{
		"namespace": namespace, "pipeline": pipeline, "name": name, "label": label,
	}).MustSql()

	sub := PipelineExtensionSubscription{}
	err := conn.Get(&sub, query, args...)
	if err != nil {
		if errors.Is(err, sql.ErrNoRows) {
			return PipelineExtensionSubscription{}, ErrEntityNotFound
		}

		return PipelineExtensionSubscription{}, fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}

	return sub, nil
}

func (db *DB) UpdatePipelineExtensionSubscription(conn Queryable, namespace, pipeline, name, label string, fields UpdateablePipelineExtensionSubscriptionFields) error {
	query := qb.Update("pipeline_extension_subscriptions")

	if fields.Settings != nil {
		query = query.Set("settings", fields.Settings)
	}

	if fields.Status != nil {
		query = query.Set("status", fields.Status)
	}

	if fields.StatusReason != nil {
		query = query.Set("status_reason", fields.StatusReason)
	}

	_, err := query.Where(qb.Eq{
		"namespace": namespace, "pipeline": pipeline, "name": name, "label": label,
	}).RunWith(conn).Exec()
	if err != nil {
		if errors.Is(err, sql.ErrNoRows) {
			return ErrEntityNotFound
		}

		return fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}

	return nil
}

func (db *DB) DeletePipelineExtensionSubscription(conn Queryable, namespace, pipeline, name, label string) error {
	_, err := qb.Delete("pipeline_extension_subscriptions").
		Where(qb.Eq{"namespace": namespace, "pipeline": pipeline, "name": name, "label": label}).RunWith(conn).Exec()
	if err != nil {
		if errors.Is(err, sql.ErrNoRows) {
			return nil
		}

		return fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}

	return nil
}
