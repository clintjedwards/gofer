package storage

import (
	"database/sql"
	"fmt"

	"github.com/jmoiron/sqlx"
	"github.com/rs/zerolog/log"
)

// migrate is a migrator that uses github.com/jmoiron/sqlx
type migrate struct {
	Migrations []migration
}

// migrate will run the migrations using the provided db connection.
func (s *migrate) migrate(db *sqlx.DB, dialect string) error {
	err := s.createMigrationTable(db)
	if err != nil {
		return err
	}
	for _, m := range s.Migrations {
		var found string
		err := db.Get(&found, "SELECT id FROM migrations WHERE id=$1", m.ID)
		switch err {
		case sql.ErrNoRows:
			log.Debug().Msgf("running migration ID: %v", m.ID)
		case nil:
			continue
		default:
			return fmt.Errorf("looking up migration by id: %w", err)
		}
		err = s.runMigration(db, m)
		if err != nil {
			return err
		}
	}
	return nil
}

func (s *migrate) createMigrationTable(db *sqlx.DB) error {
	_, err := db.Exec("CREATE TABLE IF NOT EXISTS migrations (id TEXT PRIMARY KEY )")
	if err != nil {
		return fmt.Errorf("creating migrations table: %w", err)
	}
	return nil
}

func (s *migrate) runMigration(db *sqlx.DB, m migration) error {
	errorf := func(err error) error { return fmt.Errorf("running migration: %w", err) }

	tx, err := db.Beginx()
	if err != nil {
		return errorf(err)
	}
	_, err = tx.Exec("INSERT INTO migrations (id) VALUES ($1)", m.ID)
	if err != nil {
		_ = tx.Rollback()
		return errorf(err)
	}
	err = m.Migrate(tx)
	if err != nil {
		_ = tx.Rollback()
		return errorf(err)
	}
	err = tx.Commit()
	if err != nil {
		return errorf(err)
	}
	return nil
}

// migration is a unique ID plus a function that uses a sqlx transaction
// to perform a database migration step.
type migration struct {
	ID      string
	Migrate func(tx *sqlx.Tx) error
}

// migrationQuery will create a SqlxMigration using the provided id and
// query string. It is a helper function designed to simplify the process of
// creating migrations that only depending on a SQL query string.
func migrationQuery(id, query string) migration {
	queryFn := func(query string) func(tx *sqlx.Tx) error {
		if query == "" {
			return nil
		}
		return func(tx *sqlx.Tx) error {
			_, err := tx.Exec(query)
			return err
		}
	}

	m := migration{
		ID:      id,
		Migrate: queryFn(query),
	}
	return m
}
