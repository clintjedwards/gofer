package bolt

import (
	"errors"

	"github.com/asdine/storm/v3"
	"github.com/asdine/storm/v3/q"
	"github.com/clintjedwards/gofer/internal/models"
	"github.com/clintjedwards/gofer/internal/storage"
)

func (db *DB) GetAllTokens(r storage.GetAllTokensRequest) ([]*models.Token, error) {
	if r.Limit == 0 || r.Limit > db.maxResultsLimit {
		r.Limit = db.maxResultsLimit
	}

	tokens := []*models.Token{}
	if len(r.Namespaces) != 0 {
		query := db.Select(q.In("Namespaces", r.Namespaces)).Limit(r.Limit).Skip(r.Offset)
		err := query.Find(tokens)
		if err != nil {
			return nil, err
		}

		return tokens, nil
	}

	err := db.All(&tokens, storm.Limit(r.Limit), storm.Skip(r.Offset))
	if err != nil {
		return nil, err
	}

	return tokens, nil
}

// GetToken returns a single token by hash.
func (db *DB) GetToken(r storage.GetTokenRequest) (*models.Token, error) {
	var token models.Token
	err := db.One("Hash", r.Hash, &token)
	if err != nil {
		if errors.Is(err, storm.ErrNotFound) {
			return nil, storage.ErrEntityNotFound
		}

		return nil, err
	}

	return &token, nil
}

func (db *DB) AddToken(r storage.AddTokenRequest) error {
	tx, err := db.Begin(true)
	if err != nil {
		return err
	}
	defer tx.Rollback() // nolint: errcheck

	var token models.Token
	err = tx.One("Hash", r.Token.Hash, &token)
	if errors.Is(err, storm.ErrNotFound) {
		err = tx.Save(r.Token)
		if err != nil {
			if errors.Is(err, storm.ErrNoID) {
				return storage.ErrPreconditionFailure
			}

			return err
		}

		return tx.Commit()
	}

	if err == nil {
		return storage.ErrEntityExists
	}

	return err
}

func (db *DB) DeleteToken(r storage.DeleteTokenRequest) error {
	err := db.DeleteStruct(&models.Token{Hash: r.Hash})
	if err != nil {
		if errors.Is(err, storm.ErrNotFound) {
			return storage.ErrEntityNotFound
		}

		if errors.Is(err, storm.ErrNoID) {
			return storage.ErrPreconditionFailure
		}

		return err
	}

	return nil
}
