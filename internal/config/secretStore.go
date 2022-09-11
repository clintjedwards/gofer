package config

// SqliteSecret
type SqliteSecret struct {
	Path string `hcl:"path,optional"` // file path for database file
	// EncryptionKey is a 32-bit random string of characters used to encrypt data at rest.
	EncryptionKey string `split_words:"true" hcl:"encryption_key,optional"`
}

// SecretStore defines the configuration for Gofer's secret backend.
type SecretStore struct {
	// The ObjectStore engine used by the backend.
	// Possible values are: sqlite
	Engine string `hcl:"engine,optional"`

	Sqlite *SqliteSecret `hcl:"sqlite,block"`
}

func DefaultSecretStoreConfig() *SecretStore {
	return &SecretStore{
		Engine: "sqlite",
		Sqlite: &SqliteSecret{
			Path:          "/tmp/gofer-secret.db",
			EncryptionKey: "changemechangemechangemechangeme",
		},
	}
}
