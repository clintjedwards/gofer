package storage

import (
	"database/sql"
	"errors"
	"fmt"
	"strings"

	qb "github.com/Masterminds/squirrel"
)

type Token struct {
	ID         int64
	Hash       string
	Created    int64
	Kind       string
	Namespaces string
	Metadata   string
	Expires    int64
	Disabled   bool
}

func (db *DB) ListTokens(conn Queryable, offset, limit int) ([]Token, error) {
	if limit == 0 || limit > db.maxResultsLimit {
		limit = db.maxResultsLimit
	}

	query, args := qb.Select("id", "hash", "created", "kind", "namespaces", "metadata", "expires", "disabled").
		From("tokens").
		Limit(uint64(limit)).
		Offset(uint64(offset)).MustSql()

	tokens := []Token{}
	err := conn.Select(&tokens, query, args...)
	if err != nil {
		return nil, fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}

	return tokens, nil
}

func (db *DB) InsertToken(conn Queryable, tr *Token) (int64, error) {
	result, err := qb.Insert("tokens").Columns("hash", "created", "kind", "namespaces", "metadata", "expires", "disabled").
		Values(tr.Hash, tr.Created, tr.Kind, tr.Namespaces, tr.Metadata, tr.Expires, tr.Disabled).RunWith(conn).Exec()
	if err != nil {
		if strings.Contains(err.Error(), "UNIQUE constraint failed") {
			return 0, ErrEntityExists
		}

		return 0, fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}

	return result.LastInsertId()
}

func (db *DB) GetTokenByID(conn Queryable, id int64) (Token, error) {
	query, args := qb.Select("id", "hash", "created", "kind", "namespaces", "metadata", "expires", "disabled").
		From("tokens").Where(qb.Eq{"id": id}).MustSql()

	token := Token{}
	err := conn.Get(&token, query, args...)
	if err != nil {
		if errors.Is(err, sql.ErrNoRows) {
			return Token{}, ErrEntityNotFound
		}

		return Token{}, fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}

	return token, nil
}

func (db *DB) GetTokenByHash(conn Queryable, hashStr string) (Token, error) {
	query, args := qb.Select("id", "hash", "created", "kind", "namespaces", "metadata", "expires", "disabled").
		From("tokens").Where(qb.Eq{"hash": hashStr}).MustSql()

	token := Token{}
	err := conn.Get(&token, query, args...)
	if err != nil {
		if errors.Is(err, sql.ErrNoRows) {
			return Token{}, ErrEntityNotFound
		}

		return Token{}, fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}

	return token, nil
}

func (db *DB) EnableToken(conn Queryable, hashStr string) error {
	query := qb.Update("tokens")
	query = query.Set("disabled", false)
	_, err := query.Where(qb.Eq{"hash": hashStr}).RunWith(conn).Exec()
	if err != nil {
		if strings.Contains(err.Error(), "no rows in result set") {
			return ErrEntityNotFound
		}

		return fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}

	return nil
}

func (db *DB) DisableToken(conn Queryable, hashStr string) error {
	query := qb.Update("tokens")
	query = query.Set("disabled", true)
	_, err := query.Where(qb.Eq{"hash": hashStr}).RunWith(conn).Exec()
	if err != nil {
		if strings.Contains(err.Error(), "no rows in result set") {
			return ErrEntityNotFound
		}

		return fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}

	return nil
}

func (db *DB) DeleteTokenByHash(conn Queryable, hash string) error {
	_, err := qb.Delete("tokens").Where(qb.Eq{"hash": hash}).RunWith(conn).Exec()
	if err != nil {
		if errors.Is(err, sql.ErrNoRows) {
			return nil
		}

		return fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}

	return nil
}

func (db *DB) DeleteTokenByID(conn Queryable, id int64) error {
	_, err := qb.Delete("tokens").Where(qb.Eq{"id": id}).RunWith(conn).Exec()
	if err != nil {
		if errors.Is(err, sql.ErrNoRows) {
			return nil
		}

		return fmt.Errorf("database error occurred: %v; %w", err, ErrInternal)
	}

	return nil
}
