package bolt

import (
	"errors"
	"time"

	"github.com/asdine/storm/v3"
	objectstore "github.com/clintjedwards/gofer/internal/objectStore"
	bolt "go.etcd.io/bbolt"
)

// Store is a representation of the bolt datastore
type Store struct {
	*storm.DB
}

const rootBucket string = "root"

// New creates a new boltdb with given settings
func New(path string) (Store, error) {
	store, err := storm.Open(path, storm.BoltOptions(0600, &bolt.Options{Timeout: 1 * time.Second}))
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
			return nil, objectstore.ErrEntityNotFound
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

func (store *Store) DeleteObject(key string) error {
	err := store.Delete(rootBucket, key)
	if err != nil {
		if errors.Is(err, storm.ErrNotFound) {
			return objectstore.ErrEntityNotFound
		}

		return err
	}

	return nil
}
