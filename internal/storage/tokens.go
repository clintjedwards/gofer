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

func (db *DB) ListTokens(offset, limit int) ([]models.Token, error) {
	if limit == 0 || limit > db.maxResultsLimit {
		limit = db.maxResultsLimit
	}

	rows, err := qb.Select("hash", "created", "kind", "namespaces", "metadata", "expires", "disabled").
		From("tokens").
		Limit(uint64(limit)).
		Offset(uint64(offset)).RunWith(db).Query()
	if err != nil {
		return nil, fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}
	err = rows.Err()
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
		var expires int64
		var disabled bool

		err = rows.Scan(&hash, &created, &kind, &namespacesJSON, &metadataJSON, &expires, &disabled)
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
		token.Expires = expires
		token.Disabled = disabled

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

	_, err = qb.Insert("tokens").Columns("hash", "created", "kind", "namespaces", "metadata", "expires", "disabled").
		Values(tr.Hash, tr.Created, tr.Kind, string(namespacesJSON), string(metadataJSON), tr.Expires, tr.Disabled).RunWith(db).Exec()
	if err != nil {
		if strings.Contains(err.Error(), "UNIQUE constraint failed") {
			return ErrEntityExists
		}

		return fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}

	return nil
}

func (db *DB) GetToken(hashStr string) (models.Token, error) {
	row := qb.Select("hash", "created", "kind", "namespaces", "metadata", "expires", "disabled").
		From("tokens").Where(qb.Eq{"hash": hashStr}).RunWith(db).QueryRow()

	token := models.Token{}

	var hash string
	var created int64
	var kind string
	var namespacesJSON string
	var metadataJSON string
	var expires int64
	var disabled bool

	err := row.Scan(&hash, &created, &kind, &namespacesJSON, &metadataJSON, &expires, &disabled)
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
	token.Expires = expires
	token.Disabled = disabled

	return token, nil
}

func (db *DB) EnableToken(hashStr string) error {
	query := qb.Update("tokens")
	query = query.Set("disabled", false)
	_, err := query.Where(qb.Eq{"hash": hashStr}).RunWith(db).Exec()
	if err != nil {
		if strings.Contains(err.Error(), "no rows in result set") {
			return ErrEntityNotFound
		}

		return fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}

	return nil
}

func (db *DB) DisableToken(hashStr string) error {
	query := qb.Update("tokens")
	query = query.Set("disabled", true)
	_, err := query.Where(qb.Eq{"hash": hashStr}).RunWith(db).Exec()
	if err != nil {
		if strings.Contains(err.Error(), "no rows in result set") {
			return ErrEntityNotFound
		}

		return fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}

	return nil
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
