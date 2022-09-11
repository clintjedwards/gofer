package sqlite

import (
	"crypto/aes"
	"crypto/cipher"
	"crypto/rand"
	"database/sql"
	"errors"
	"fmt"
	"io"
	"strings"

	qb "github.com/Masterminds/squirrel"
	"github.com/clintjedwards/gofer/internal/secretStore"
	"github.com/jmoiron/sqlx"
	_ "github.com/mattn/go-sqlite3" // Provides sqlite3 lib
	"github.com/rs/zerolog/log"
)

const initialMigration = `CREATE TABLE IF NOT EXISTS secrets (
    key         TEXT    NOT NULL,
    value       BLOB    NOT NULL,
    PRIMARY KEY (key)
) STRICT;`

// Store is a representation of the bolt datastore
type Store struct {
	encryptionKey string
	*sqlx.DB
}

// New creates a new boltdb with given settings
func New(path, encryptionKey string) (Store, error) {
	dsn := fmt.Sprintf("%s?_journal=wal&_fk=true&_timeout=5000", path)

	db, err := sqlx.Connect("sqlite3", dsn)
	if err != nil {
		return Store{}, err
	}

	_ = db.MustExec(initialMigration)

	return Store{
		encryptionKey,
		db,
	}, nil
}

func encrypt(key []byte, plaintext []byte) ([]byte, error) {
	c, err := aes.NewCipher(key)
	if err != nil {
		return nil, err
	}

	gcm, err := cipher.NewGCM(c)
	if err != nil {
		return nil, err
	}

	nonce := make([]byte, gcm.NonceSize())
	if _, err = io.ReadFull(rand.Reader, nonce); err != nil {
		return nil, err
	}

	return gcm.Seal(nonce, nonce, plaintext, nil), nil
}

func decrypt(key []byte, ciphertext []byte) ([]byte, error) {
	c, err := aes.NewCipher(key)
	if err != nil {
		return nil, err
	}

	gcm, err := cipher.NewGCM(c)
	if err != nil {
		return nil, err
	}

	nonceSize := gcm.NonceSize()
	if len(ciphertext) < nonceSize {
		return nil, errors.New("ciphertext too short")
	}

	nonce, ciphertext := ciphertext[:nonceSize], ciphertext[nonceSize:]
	return gcm.Open(nil, nonce, ciphertext, nil)
}

func (store *Store) GetSecret(key string) (string, error) {
	row := qb.Select("value").
		From("secrets").Where(qb.Eq{"key": key}).RunWith(store).QueryRow()

	var value []byte
	err := row.Scan(&value)
	if err != nil {
		if errors.Is(err, sql.ErrNoRows) {
			return "", secretStore.ErrEntityNotFound
		}

		return "", fmt.Errorf("database error occurred: %v; %w", err, secretStore.ErrInternal)
	}

	decryptedSecret, err := decrypt([]byte(store.encryptionKey), value)
	if err != nil {
		log.Error().Err(err).Msg("could not decrypt secret")
		return "", err
	}

	return string(decryptedSecret), nil
}

func (store *Store) ListSecretKeys(prefix string) ([]string, error) {
	rows, err := qb.Select("key").
		From("secrets").Where(qb.Like{"key": prefix + "%"}).RunWith(store).Query()
	if err != nil {
		return nil, fmt.Errorf("database error occurred: %v; %w", err, secretStore.ErrInternal)
	}
	defer rows.Close()

	keys := []string{}

	for rows.Next() {
		var key string

		err = rows.Scan(&key)
		if err != nil {
			return nil, fmt.Errorf("database error occurred: %v; %w", err, secretStore.ErrInternal)
		}

		keys = append(keys, key)
	}
	err = rows.Err()
	if err != nil {
		return nil, fmt.Errorf("database error occurred: %v; %w", err, secretStore.ErrInternal)
	}

	return keys, nil
}

func (store *Store) PutSecret(key string, content string, force bool) error {
	encryptedSecret, err := encrypt([]byte(store.encryptionKey), []byte(content))
	if err != nil {
		log.Error().Err(err).Msg("could not encrypt secret")
		return fmt.Errorf("could not encrypt secret")
	}

	_, err = qb.Insert("secrets").Columns("key", "value").Values(key, encryptedSecret).RunWith(store).Exec()
	if err != nil {
		if strings.Contains(err.Error(), "UNIQUE constraint failed") {
			return secretStore.ErrEntityExists
		}

		return fmt.Errorf("database error occurred: %v; %w", err, secretStore.ErrInternal)
	}

	return nil
}

func (store *Store) DeleteSecret(key string) error {
	_, err := qb.Delete("secrets").Where(qb.Eq{"key": key}).RunWith(store).Exec()
	if err != nil {
		if errors.Is(err, sql.ErrNoRows) {
			return nil
		}

		return fmt.Errorf("database error occurred: %v; %w", err, secretStore.ErrInternal)
	}

	return nil
}
