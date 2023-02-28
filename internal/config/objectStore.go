package config

// ObjectStore defines config settings for gofer ObjectStore. The ObjectStore stores temporary objects for pipelines and
// runs.
type ObjectStore struct {
	// The ObjectStore engine used by the backend.
	// Possible values are: bolt
	Engine string `koanf:"engine"`

	Sqlite *Sqlite `koanf:"sqlite"`

	// Pipeline Objects last forever but are limited in number. This is the total amount of items that can be stored
	// per pipeline before gofer starts deleting objects.
	PipelineObjectLimit int `koanf:"pipeline_object_limit"`

	// Objects stored at the run level are unlimited in number, but only last for a certain number of runs.
	// The number below controls how many runs until the run objects for the oldest run will be deleted.
	// Ex. an object stored on run number #5 with an expiry of 2 will be deleted on run #7 regardless of run
	// health.
	RunObjectExpiry int `koanf:"run_object_expiry"`
}

// Sqlite
type Sqlite struct {
	Path string `koanf:"path"` // file path for database file
}

func DefaultObjectStoreConfig() *ObjectStore {
	return &ObjectStore{
		Engine: "sqlite",
		Sqlite: &Sqlite{
			Path: "/tmp/gofer-object.db",
		},
		PipelineObjectLimit: 50,
		RunObjectExpiry:     50,
	}
}
