// Package storage contains the data storage interface in which Gofer stores all internal data.
package storage

import (
	"context"
	"database/sql"
	"embed"
	"errors"
	"fmt"

	"github.com/jmoiron/sqlx"
	_ "github.com/mattn/go-sqlite3" // Provides sqlite3 lib
	"github.com/rs/zerolog/log"
)

//go:embed migrations
var migrations embed.FS

var (
	// ErrEntityNotFound is returned when a certain entity could not be located.
	ErrEntityNotFound = errors.New("storage: entity not found")

	// ErrEntityExists is returned when a certain entity was located but not meant to be.
	ErrEntityExists = errors.New("storage: entity already exists")

	// ErrPreconditionFailure is returned when there was a validation error with the parameters passed.
	ErrPreconditionFailure = errors.New("storage: parameters did not pass validation")

	// ErrInternal is returned when there was an unknown internal DB error.
	ErrInternal = errors.New("storage: unknown db error")
)

// Queryable includes methods shared by sqlx.Tx and sqlx.DB so they can
// be used interchangeably.
type Queryable interface {
	sqlx.Queryer
	sqlx.Execer
	GetContext(context.Context, interface{}, string, ...interface{}) error
	SelectContext(context.Context, interface{}, string, ...interface{}) error
	Get(interface{}, string, ...interface{}) error
	MustExecContext(context.Context, string, ...interface{}) sql.Result
	PreparexContext(context.Context, string) (*sqlx.Stmt, error)
	QueryRowContext(context.Context, string, ...interface{}) *sql.Row
	Select(interface{}, string, ...interface{}) error
	QueryRow(string, ...interface{}) *sql.Row
	PrepareNamedContext(context.Context, string) (*sqlx.NamedStmt, error)
	PrepareNamed(string) (*sqlx.NamedStmt, error)
	Preparex(string) (*sqlx.Stmt, error)
	NamedExec(string, interface{}) (sql.Result, error)
	NamedExecContext(context.Context, string, interface{}) (sql.Result, error)
	MustExec(string, ...interface{}) sql.Result
	NamedQuery(string, interface{}) (*sqlx.Rows, error)
}

// DB is a representation of the datastore
type DB struct {
	maxResultsLimit int
	*sqlx.DB
}

func mustReadFile(path string) []byte {
	file, err := migrations.ReadFile(path)
	if err != nil {
		log.Fatal().Err(err).Msg("could not read migrations file")
	}

	return file
}

// New creates a new db with given settings
func New(path string, maxResultsLimit int) (DB, error) {
	dsn := fmt.Sprintf("%s?_journal=wal&_fk=true&_timeout=5000", path)

	db, err := sqlx.Connect("sqlite3", dsn)
	if err != nil {
		return DB{}, err
	}

	migration := migrate{
		Migrations: []migration{
			migrationQuery("0", string(mustReadFile("migrations/0_init.sql"))),
		},
	}

	err = migration.migrate(db, "sqlite3")
	if err != nil {
		return DB{}, err
	}

	return DB{
		maxResultsLimit,
		db,
	}, nil
}

// InsideTx is a convenience function so that callers can run multiple queries inside a transaction.
func InsideTx(db *sqlx.DB, fn func(*sqlx.Tx) error) error {
	tx, err := db.Beginx()
	if err != nil {
		return err
	}

	defer func() {
		if v := recover(); v != nil {
			_ = tx.Rollback()
			panic(v)
		}
	}()

	if err := fn(tx); err != nil {
		if rerr := tx.Rollback(); rerr != nil {
			err = fmt.Errorf("%w: rolling back transaction: %v", err, rerr)
		}
		return err
	}

	if err := tx.Commit(); err != nil {
		return fmt.Errorf("committing transaction: %w", err)
	}

	return nil
}
