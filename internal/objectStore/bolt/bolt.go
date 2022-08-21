package bolt

import (
	"bytes"
	"errors"
	"time"

	"github.com/asdine/storm/v3"
	"github.com/clintjedwards/gofer/internal/objectStore"
	"github.com/rs/zerolog/log"
	bolt "go.etcd.io/bbolt"
)

// Store is a representation of the bolt datastore
type Store struct {
	*storm.DB
}

const rootBucket string = "root"

// New creates a new boltdb with given settings
func New(path string) (Store, error) {
	store, err := storm.Open(path, storm.BoltOptions(0o600, &bolt.Options{Timeout: 1 * time.Second}))
	if err != nil {
		return Store{}, err
	}

	return Store{
		store,
	}, nil
}

func (store *Store) GetObject(key string) ([]byte, error) {
	var storedObject []byte

	err := store.Get(rootBucket, key, &storedObject)
	if err != nil {
		if errors.Is(err, storm.ErrNotFound) {
			return nil, objectStore.ErrEntityNotFound
		}

		return nil, err
	}

	return storedObject, nil
}

func (store *Store) PutObject(key string, content []byte, force bool) error {
	err := store.Set(rootBucket, key, content)
	if err != nil {
		return err
	}

	return nil
}

func (store *Store) ListObjectKeys(prefix string) ([]string, error) {
	keys := []string{}

	err := store.Bolt.View(func(tx *bolt.Tx) error {
		bucket := tx.Bucket([]byte(rootBucket)).Cursor()

		for key, _ := bucket.Seek([]byte(prefix)); key != nil && bytes.HasPrefix(key, []byte(prefix)); key, _ = bucket.Next() {
			keys = append(keys, string(key))
		}

		return nil
	})
	if err != nil {
		log.Error().Err(err).Msg("could not list object keys")
		return nil, err
	}

	return keys, nil
}

func (store *Store) DeleteObject(key string) error {
	err := store.Delete(rootBucket, key)
	if err != nil {
		if errors.Is(err, storm.ErrNotFound) {
			return objectStore.ErrEntityNotFound
		}

		return err
	}

	return nil
}
