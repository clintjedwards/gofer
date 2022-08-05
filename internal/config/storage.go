package config

// Database defines config settings for gofer database
type Database struct {
	// MaxResultsLimit defines the total number of results the database can return in one call to any "GETALL" endpoint.
	MaxResultsLimit int    `split_words:"true" hcl:"max_results_limit,optional"`
	Path            string `hcl:"path,optional"`
}

func DefaultDatabaseConfig() *Database {
	return &Database{
		Path:            "/tmp/gofer.db",
		MaxResultsLimit: 100,
	}
}
