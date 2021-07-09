package bolt

import (
	"bytes"
	"encoding/json"

	"github.com/clintjedwards/gofer/internal/storage"
	"github.com/rs/zerolog/log"
	bolt "go.etcd.io/bbolt"
)

const runRegistryBucketName = "runRegistry"

func (db *DB) GetAllRunRegistrations(r storage.GetAllRunRegistrationsRequest) (map[storage.RunRegistryKey]struct{}, error) {
	results := map[storage.RunRegistryKey]struct{}{}

	err := db.Bolt.View(func(tx *bolt.Tx) error {
		bucket := tx.Bucket([]byte(runRegistryBucketName))

		err := bucket.ForEach(func(key, _ []byte) error {
			// Storm stores some metadata about buckets in keys that start with __
			// We want to avoid these keys since they are not in the format we're expecting.
			if bytes.HasPrefix(key, []byte("__")) {
				return nil
			}

			var runlogkey storage.RunRegistryKey

			err := json.Unmarshal(key, &runlogkey)
			if err != nil {
				log.Error().Err(err).Str("id", string(key)).Msg("could not unmarshal database object")
				return err
			}

			results[runlogkey] = struct{}{}
			return nil
		})
		if err != nil {
			return err
		}

		return nil
	})
	if err != nil {
		return nil, err
	}

	return results, nil
}

func (db *DB) RegistrationExists(r storage.RegistrationExistsRequest) bool {
	exists, err := db.KeyExists(runRegistryBucketName, storage.RunRegistryKey(r))
	if err != nil {
		return false
	}

	return exists
}

func (db *DB) RegisterRun(r storage.RegisterRunRequest) error {
	err := db.Set(runRegistryBucketName, storage.RunRegistryKey(r), struct{}{})
	if err != nil {
		return err
	}

	return nil
}

func (db *DB) UnregisterRun(r storage.UnregisterRunRequest) error {
	err := db.Delete(runRegistryBucketName, storage.RunRegistryKey(r))
	if err != nil {
		return err
	}

	return nil
}
