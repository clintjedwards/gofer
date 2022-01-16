package config

// SecretStore defines the configuration for Gofer's secret backend.
type SecretStore struct {
	// The ObjectStore engine used by the backend.
	// Possible values are: bolt
	Engine string `hcl:"engine,optional"`

	BoltDB *BoltDB `hcl:"boltdb,block"`

	// EncryptionKey is a 32-bit random string of characters used to encrypt data at rest.
	EncryptionKey string `split_words:"true" hcl:"encryption_key,optional"`
}

func DefaultSecretStoreConfig() *SecretStore {
	return &SecretStore{
		Engine: "bolt",
		BoltDB: &BoltDB{
			Path: "/tmp/gofer-secret.db",
		},
		EncryptionKey: "changemechangemechangemechangeme",
	}
}
