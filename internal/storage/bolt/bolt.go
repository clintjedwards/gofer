package bolt

import (
	"time"

	"github.com/asdine/storm/v3"
	bolt "go.etcd.io/bbolt"
)

// DB is a representation of the bolt datastore
type DB struct {
	maxResultsLimit int
	*storm.DB
}

// New creates a new boltdb with given settings
func New(path string, maxResultsLimit int) (DB, error) {
	store, err := storm.Open(path, storm.BoltOptions(0600, &bolt.Options{Timeout: 1 * time.Second}))
	if err != nil {
		return DB{}, err
	}

	err = store.Bolt.Update(func(tx *bolt.Tx) error {
		_, err := store.CreateBucketIfNotExists(tx, runRegistryBucketName)
		if err != nil {
			return err
		}

		return nil
	})
	if err != nil {
		return DB{}, err
	}

	return DB{
		maxResultsLimit,
		store,
	}, nil
}
