package config

// ObjectStore defines config settings for gofer ObjectStore. The ObjectStore stores temporary objects for pipelines and
// runs.
type ObjectStore struct {
	// The ObjectStore engine used by the backend.
	// Possible values are: bolt
	Engine string `hcl:"engine,optional"`

	BoltDB *BoltDB `hcl:"boltdb,block"`

	// Pipeline Objects last forever but are limited in number. This is the total amount of items that can be stored
	// per pipeline before gofer starts deleting objects.
	PipelineObjectLimit int `split_words:"true" hcl:"pipeline_object_limit,optional"`

	// Objects stored at the run level are unlimited in number, but only last for a certain number of runs.
	// The number below controls how many runs until the run objects for the oldest run will be deleted.
	// Ex. an object stored on run number #5 with an expiry of 2 will be deleted on run #7 regardless of run
	// health.
	RunObjectExpiry int `split_words:"true" hcl:"run_object_expiry,optional"`
}

func DefaultObjectStoreConfig() *ObjectStore {
	return &ObjectStore{
		Engine: "bolt",
		BoltDB: &BoltDB{
			Path: "/tmp/gofer-os.db",
		},
		PipelineObjectLimit: 10,
		RunObjectExpiry:     20,
	}
}
