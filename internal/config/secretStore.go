package config

// BoltDBSecret: https://pkg.go.dev/go.etcd.io/bbolt
type BoltDBSecret struct {
	Path string `hcl:"path,optional"` // file path for database file
	// EncryptionKey is a 32-bit random string of characters used to encrypt data at rest.
	EncryptionKey string `split_words:"true" hcl:"encryption_key,optional"`
}

// SecretStore defines the configuration for Gofer's secret backend.
type SecretStore struct {
	// The ObjectStore engine used by the backend.
	// Possible values are: bolt
	Engine string `hcl:"engine,optional"`

	BoltDB *BoltDBSecret `hcl:"boltdb,block"`
}

func DefaultSecretStoreConfig() *SecretStore {
	return &SecretStore{
		Engine: "bolt",
		BoltDB: &BoltDBSecret{
			Path:          "/tmp/gofer-secret.db",
			EncryptionKey: "changemechangemechangemechangeme",
		},
	}
}
