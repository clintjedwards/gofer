package config

// Database defines config settings for gofer database
type Database struct {
	// The database engine used by the backend
	// possible values are: bolt
	Engine string `hcl:"engine,optional"`

	// MaxResultsLimit defines the total number of results the database can return in one call to any "GETALL" endpoint.
	MaxResultsLimit int     `split_words:"true" hcl:"max_results_limit,optional"`
	BoltDB          *BoltDB `hcl:"boltdb,block"`
}

func DefaultDatabaseConfig() *Database {
	return &Database{
		Engine:          "bolt",
		MaxResultsLimit: 100,
		BoltDB: &BoltDB{
			Path: "/tmp/gofer.db",
		},
	}
}

// BoltDB: https://pkg.go.dev/go.etcd.io/bbolt
type BoltDB struct {
	Path string `hcl:"path,optional"` // file path for database file
}
