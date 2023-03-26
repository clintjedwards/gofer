package config

import (
	"os"
	"testing"
	"time"

	"github.com/google/go-cmp/cmp"
)

func TestInitAPIConfigAgainstSample(t *testing.T) {
	config, err := InitAPIConfig("../cli/service/sampleConfig.hcl", false, false, false)
	if err != nil {
		t.Fatal(err)
	}

	expected := API{
		IgnorePipelineRunEvents: false,
		RunParallelismLimit:     200,
		PipelineVersionLimit:    5,
		EventLogRetention:       time.Hour * 4380,
		EventPruneInterval:      time.Hour * 3,
		LogLevel:                "info",
		TaskRunLogExpiry:        50,
		TaskRunLogsDir:          "/tmp",
		TaskRunStopTimeout:      time.Minute * 5,

		ExternalEventsAPI: &ExternalEventsAPI{
			Enable: true,
			Host:   "localhost:8081",
		},

		ObjectStore: &ObjectStore{
			Engine: "sqlite",
			Sqlite: &Sqlite{
				Path: "/tmp/gofer-object.db",
			},
			PipelineObjectLimit: 50,
			RunObjectExpiry:     50,
		},

		SecretStore: &SecretStore{
			Engine: "sqlite",
			Sqlite: &SqliteSecret{
				Path:          "/tmp/gofer-secret.db",
				EncryptionKey: "changemechangemechangemechangeme",
			},
		},

		Scheduler: &Scheduler{
			Engine: "docker",
			Docker: &Docker{
				Prune:         true,
				PruneInterval: time.Hour * 24,
			},
		},

		Server: &Server{
			Host:                "localhost:8080",
			ShutdownTimeout:     time.Second * 15,
			TLSCertPath:         "./localhost.crt",
			TLSKeyPath:          "./localhost.key",
			StoragePath:         "/tmp/gofer.db",
			StorageResultsLimit: 200,
		},

		Extensions: &Extensions{
			InstallBaseExtensions: true,
			StopTimeout:           time.Minute * 5,
			TLSCertPath:           "./localhost.crt",
			TLSKeyPath:            "./localhost.key",
		},
	}

	diff := cmp.Diff(expected, *config)
	if diff != "" {
		t.Errorf("result is different than expected(-want +got):\n%s", diff)
	}
}

func TestInitAPIConfigAgainstSampleOverwriteWithEnvs(t *testing.T) {
	_ = os.Setenv("GOFER_IGNORE_PIPELINE_RUN_EVENTS", "true")
	_ = os.Setenv("GOFER_EXTERNAL_EVENTS_API__ENABLE", "true")
	_ = os.Setenv("GOFER_OBJECT_STORE__RUN_OBJECT_EXPIRY", "1000")
	_ = os.Setenv("GOFER_SCHEDULER__DOCKER__PRUNE", "false")
	_ = os.Setenv("GOFER_SERVER__TLS_CERT_PATH", "./test")
	_ = os.Setenv("GOFER_EXTENSIONS__TLS_CERT_PATH", "./test")
	defer os.Unsetenv("GOFER_IGNORE_PIPELINE_RUN_EVENTS")
	defer os.Unsetenv("GOFER_EXTERNAL_EVENTS_API__ENABLE")
	defer os.Unsetenv("GOFER_OBJECT_STORE__RUN_OBJECT_EXPIRY")
	defer os.Unsetenv("GOFER_SCHEDULER__DOCKER__PRUNE")
	defer os.Unsetenv("GOFER_SERVER__TLS_CERT_PATH")
	defer os.Unsetenv("GOFER_EXTENSIONS__TLS_CERT_PATH")

	config, err := InitAPIConfig("../cli/service/sampleConfig.hcl", false, false, false)
	if err != nil {
		t.Fatal(err)
	}

	expected := API{
		IgnorePipelineRunEvents: true,
		RunParallelismLimit:     200,
		PipelineVersionLimit:    5,
		EventLogRetention:       time.Hour * 4380,
		EventPruneInterval:      time.Hour * 3,
		LogLevel:                "info",
		TaskRunLogExpiry:        50,
		TaskRunLogsDir:          "/tmp",
		TaskRunStopTimeout:      time.Minute * 5,

		ExternalEventsAPI: &ExternalEventsAPI{
			Enable: true,
			Host:   "localhost:8081",
		},

		ObjectStore: &ObjectStore{
			Engine: "sqlite",
			Sqlite: &Sqlite{
				Path: "/tmp/gofer-object.db",
			},
			PipelineObjectLimit: 50,
			RunObjectExpiry:     1000,
		},

		SecretStore: &SecretStore{
			Engine: "sqlite",
			Sqlite: &SqliteSecret{
				Path:          "/tmp/gofer-secret.db",
				EncryptionKey: "changemechangemechangemechangeme",
			},
		},

		Scheduler: &Scheduler{
			Engine: "docker",
			Docker: &Docker{
				Prune:         false,
				PruneInterval: time.Hour * 24,
			},
		},

		Server: &Server{
			Host:                "localhost:8080",
			ShutdownTimeout:     time.Second * 15,
			TLSCertPath:         "./test",
			TLSKeyPath:          "./localhost.key",
			StoragePath:         "/tmp/gofer.db",
			StorageResultsLimit: 200,
		},

		Extensions: &Extensions{
			InstallBaseExtensions: true,
			StopTimeout:           time.Minute * 5,
			TLSCertPath:           "./test",
			TLSKeyPath:            "./localhost.key",
		},
	}

	diff := cmp.Diff(expected, *config)
	if diff != "" {
		t.Errorf("result is different than expected(-want +got):\n%s", diff)
	}
}
