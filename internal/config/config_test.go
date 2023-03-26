package config

import (
	"testing"

	"github.com/fatih/structs"
)

// Simply test for panics, the reflect code here will panic if the API struct has any
// pointers with zero values.
func TestGetEnvvarsFromStruct(t *testing.T) {
	api := API{
		Development:       &Development{},
		ExternalEventsAPI: &ExternalEventsAPI{},
		ObjectStore: &ObjectStore{
			Sqlite: &Sqlite{},
		},
		SecretStore: &SecretStore{
			Sqlite: &SqliteSecret{},
		},
		Scheduler: &Scheduler{
			Docker: &Docker{},
		},
		Server:     &Server{},
		Extensions: &Extensions{},
	}
	fields := structs.Fields(api)
	getEnvVarsFromStruct("GOFER_", fields)
}
