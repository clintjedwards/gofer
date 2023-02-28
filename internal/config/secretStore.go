package config

// SqliteSecret
type SqliteSecret struct {
	Path string `koanf:"path"` // file path for database file
	// EncryptionKey is a 32-bit random string of characters used to encrypt data at rest.
	EncryptionKey string `split_words:"true" koanf:"encryption_key"`
}

// SecretStore defines the configuration for Gofer's secret backend.
type SecretStore struct {
	// The ObjectStore engine used by the backend.
	// Possible values are: sqlite
	Engine string `koanf:"engine"`

	Sqlite *SqliteSecret `koanf:"sqlite"`
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
