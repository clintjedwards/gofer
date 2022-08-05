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

// CREATE TABLE IF NOT EXISTS tokens (
//     hash        TEXT NOT NULL,
//     created     INTEGER NOT NULL,
//     kind        TEXT NOT NULL,
//     namespaces  TEXT NOT NULL,
//     metadata    TEXT,
//     PRIMARY KEY (hash)
// ) STRICT;

func (db *DB) ListTokens(offset, limit int) ([]models.Token, error) {
	if limit == 0 || limit > db.maxResultsLimit {
		limit = db.maxResultsLimit
	}

	rows, err := qb.Select("hash", "created", "kind", "namespaces", "metadata").
		From("tokens").
		Limit(uint64(limit)).
		Offset(uint64(offset)).RunWith(db).Query()
	if err != nil {
		return nil, fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}
	defer rows.Close()

	tokens := []models.Token{}

	for rows.Next() {
		token := models.Token{}

		var hash string
		var created int64
		var kind string
		var namespacesJSON string
		var metadataJSON string

		err = rows.Scan(&hash, &created, &kind, &namespacesJSON, &metadataJSON)
		if err != nil {
			return nil, fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
		}

		namespaces := []string{}
		err = json.Unmarshal([]byte(namespacesJSON), &namespaces)
		if err != nil {
			return nil, fmt.Errorf("database error occurred; could not decode object; %v", err)
		}

		metadata := map[string]string{}
		err = json.Unmarshal([]byte(metadataJSON), &metadata)
		if err != nil {
			return nil, fmt.Errorf("database error occurred; could not decode object; %v", err)
		}

		token.Hash = hash
		token.Created = created
		token.Kind = models.TokenKind(kind)
		token.Namespaces = namespaces
		token.Metadata = metadata

		tokens = append(tokens, token)
	}
	err = rows.Err()
	if err != nil {
		return nil, fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}

	return tokens, nil
}

func (db *DB) InsertToken(tr *models.Token) error {
	namespacesJSON, err := json.Marshal(tr.Namespaces)
	if err != nil {
		return fmt.Errorf("database error occurred; could not encode object; %v", err)
	}

	metadataJSON, err := json.Marshal(tr.Metadata)
	if err != nil {
		return fmt.Errorf("database error occurred; could not encode object; %v", err)
	}

	_, err = qb.Insert("tokens").Columns("hash", "created", "kind", "namespaces", "metadata").
		Values(tr.Hash, tr.Created, tr.Kind, string(namespacesJSON), string(metadataJSON)).RunWith(db).Exec()
	if err != nil {
		if strings.Contains(err.Error(), "UNIQUE constraint failed") {
			return ErrEntityExists
		}

		return fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}

	return nil
}

func (db *DB) GetToken(hashStr string) (models.Token, error) {
	row := qb.Select("hash", "created", "kind", "namespaces", "metadata").
		From("tokens").Where(qb.Eq{"hash": hashStr}).RunWith(db).QueryRow()

	token := models.Token{}

	var hash string
	var created int64
	var kind string
	var namespacesJSON string
	var metadataJSON string

	err := row.Scan(&hash, &created, &kind, &namespacesJSON, &metadataJSON)
	if err != nil {
		if errors.Is(err, sql.ErrNoRows) {
			return models.Token{}, ErrEntityNotFound
		}

		return models.Token{}, fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}

	namespaces := []string{}
	err = json.Unmarshal([]byte(namespacesJSON), &namespaces)
	if err != nil {
		return models.Token{}, fmt.Errorf("database error occurred; could not decode object; %v", err)
	}

	metadata := map[string]string{}
	err = json.Unmarshal([]byte(metadataJSON), &metadata)
	if err != nil {
		return models.Token{}, fmt.Errorf("database error occurred; could not decode object; %v", err)
	}

	token.Hash = hash
	token.Created = created
	token.Kind = models.TokenKind(kind)
	token.Namespaces = namespaces
	token.Metadata = metadata

	return token, nil
}

func (db *DB) DeleteToken(hash string) error {
	_, err := qb.Delete("tokens").Where(qb.Eq{"hash": hash}).RunWith(db).Exec()
	if err != nil {
		if errors.Is(err, sql.ErrNoRows) {
			return nil
		}

		return fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}

	return nil
}
