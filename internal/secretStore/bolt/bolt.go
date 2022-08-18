package bolt

import (
	"bytes"
	"crypto/aes"
	"crypto/cipher"
	"crypto/rand"
	"errors"
	"fmt"
	"io"
	"time"

	"github.com/asdine/storm/v3"
	"github.com/clintjedwards/gofer/internal/secretStore"
	"github.com/rs/zerolog/log"
	bolt "go.etcd.io/bbolt"
)

// Store is a representation of the bolt datastore
type Store struct {
	encryptionKey string
	*storm.DB
}

const rootBucket string = "root"

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

// New creates a new boltdb with given settings
func New(path, encryptionKey string) (Store, error) {
	store, err := storm.Open(path, storm.BoltOptions(0o600, &bolt.Options{Timeout: 1 * time.Second}))
	if err != nil {
		return Store{}, err
	}

	return Store{
		encryptionKey,
		store,
	}, nil
}

func (store *Store) GetSecret(key string) (string, error) {
	var storedSecret []byte

	err := store.Get(rootBucket, key, &storedSecret)
	if err != nil {
		if errors.Is(err, storm.ErrNotFound) {
			return "", secretStore.ErrEntityNotFound
		}

		return "", err
	}

	decryptedSecret, err := decrypt([]byte(store.encryptionKey), storedSecret)
	if err != nil {
		log.Error().Err(err).Msg("could not decrypt secret")
		return "", err
	}

	return string(decryptedSecret), nil
}

// db.View(func(tx *bolt.Tx) error {
// 	// Assume bucket exists and has keys
// 	c := tx.Bucket([]byte("MyBucket")).Cursor()

// 	prefix := []byte("1234")
// 	for k, v := c.Seek(prefix); k != nil && bytes.HasPrefix(k, prefix); k, v = c.Next() {
// 		fmt.Printf("key=%s, value=%s\n", k, v)
// 	}

// 	return nil
// })

func (store *Store) ListSecretKeys(prefix string) ([]string, error) {
	keys := []string{}

	err := store.Bolt.View(func(tx *bolt.Tx) error {
		bucket := tx.Bucket([]byte(rootBucket)).Cursor()

		for key, _ := bucket.Seek([]byte(prefix)); key != nil && bytes.HasPrefix(key, []byte(prefix)); key, _ = bucket.Next() {
			keys = append(keys, string(key))
		}

		return nil
	})
	if err != nil {
		log.Error().Err(err).Msg("could not list secret keys")
		return nil, err
	}

	return keys, nil
}

func (store *Store) PutSecret(key string, content string, force bool) error {
	encryptedSecret, err := encrypt([]byte(store.encryptionKey), []byte(content))
	if err != nil {
		log.Error().Err(err).Msg("could not encrypt secret")
		return fmt.Errorf("could not encrypt secret")
	}

	tx, err := store.Begin(true)
	if err != nil {
		return err
	}
	defer tx.Rollback() // nolint: errcheck

	exists, err := tx.KeyExists(rootBucket, key)
	if err != nil {
		if errors.Is(err, storm.ErrNotFound) {
		} else {
			return err
		}
	}

	if exists && !force {
		return secretStore.ErrEntityExists
	}

	err = tx.Set(rootBucket, key, encryptedSecret)
	if err != nil {
		return err
	}

	return tx.Commit()
}

func (store *Store) DeleteSecret(key string) error {
	err := store.Delete(rootBucket, key)
	if err != nil {
		if errors.Is(err, storm.ErrNotFound) {
			return secretStore.ErrEntityNotFound
		}

		return err
	}

	return nil
}