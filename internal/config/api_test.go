package config

import (
	"os"
	"testing"
	"time"

	"github.com/google/go-cmp/cmp"
)

// Tests that our sample server config is still valid. This test catches any extraneous parameters
// due to how the HCL parsing works and should also catch any errant types.
func TestAPISampleFromFile(t *testing.T) {
	hclconf := API{}
	err := hclconf.FromFile("../cli/service/sampleConfig.hcl")
	if err != nil {
		t.Fatal(err)
	}

	expected := API{
		DevMode:                 false,
		IgnorePipelineRunEvents: false,
		PipelineVersionLimit:    5,
		EventLogRetention:       time.Hour * 4380,
		EventLogRetentionHCL:    "4380h",
		EventPruneInterval:      time.Hour * 3,
		EventPruneIntervalHCL:   "3h",
		LogLevel:                "info",
		TaskRunLogExpiry:        50,
		TaskRunLogsDir:          "/tmp",
		TaskRunStopTimeout:      time.Minute * 5,
		TaskRunStopTimeoutHCL:   "5m",

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
				Prune:            true,
				PruneInterval:    time.Hour * 24,
				PruneIntervalHCL: "24h",
			},
		},

		Server: &Server{
			Host:                "localhost:8080",
			ShutdownTimeout:     time.Second * 15,
			ShutdownTimeoutHCL:  "15s",
			TLSCertPath:         "./localhost.crt",
			TLSKeyPath:          "./localhost.key",
			StoragePath:         "/tmp/gofer.db",
			StorageResultsLimit: 200,
		},

		Extensions: &Extensions{
			InstallBaseExtensions: true,
			StopTimeout:           time.Minute * 5,
			StopTimeoutHCL:        "5m",
			TLSCertPath:           "./localhost.crt",
			TLSKeyPath:            "./localhost.key",
		},
	}

	diff := cmp.Diff(expected, hclconf)
	if diff != "" {
		t.Errorf("result is different than expected(-want +got):\n%s", diff)
	}
}

func TestAPISampleOverwriteWithEnvs(t *testing.T) {
	_ = os.Setenv("GOFER_IGNORE_PIPELINE_RUN_EVENTS", "false")
	_ = os.Setenv("GOFER_EXTERNAL_EVENTS_API_ENABLE", "false")
	_ = os.Setenv("GOFER_DATABASE_MAX_RESULTS_LIMIT", "1000")
	_ = os.Setenv("GOFER_OBJECTSTORE_RUN_OBJECT_EXPIRY", "1000")
	_ = os.Setenv("GOFER_SCHEDULER_DOCKER_PRUNE", "false")
	_ = os.Setenv("GOFER_SERVER_TLS_CERT_PATH", "./test")
	_ = os.Setenv("GOFER_EXTENSIONS_TLS_CERT_PATH", "./test")
	defer os.Unsetenv("GOFER_IGNORE_PIPELINE_RUN_EVENTS")
	defer os.Unsetenv("GOFER_EXTERNAL_EVENTS_API_ENABLE")
	defer os.Unsetenv("GOFER_DATABASE_MAX_RESULTS_LIMIT")
	defer os.Unsetenv("GOFER_OBJECTSTORE_RUN_OBJECT_EXPIRY")
	defer os.Unsetenv("GOFER_SCHEDULER_DOCKER_PRUNE")
	defer os.Unsetenv("GOFER_SERVER_TLS_CERT_PATH")
	defer os.Unsetenv("GOFER_EXTENSIONS_TLS_CERT_PATH")

	hclconf := API{}
	err := hclconf.FromFile("../cli/service/sampleConfig.hcl")
	if err != nil {
		t.Fatal(err)
	}

	err = hclconf.FromEnv()
	if err != nil {
		t.Fatal(err)
	}

	expected := API{
		DevMode:                 false,
		IgnorePipelineRunEvents: false,
		PipelineVersionLimit:    5,
		EventLogRetention:       time.Hour * 4380,
		EventLogRetentionHCL:    "4380h",
		EventPruneInterval:      time.Hour * 3,
		EventPruneIntervalHCL:   "3h",
		LogLevel:                "info",
		TaskRunLogExpiry:        50,
		TaskRunLogsDir:          "/tmp",
		TaskRunStopTimeout:      time.Minute * 5,
		TaskRunStopTimeoutHCL:   "5m",

		ExternalEventsAPI: &ExternalEventsAPI{
			Enable: false,
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
				Prune:            false,
				PruneInterval:    time.Hour * 24,
				PruneIntervalHCL: "24h",
			},
		},

		Server: &Server{
			Host:                "localhost:8080",
			ShutdownTimeout:     time.Second * 15,
			ShutdownTimeoutHCL:  "15s",
			TLSCertPath:         "./test",
			TLSKeyPath:          "./localhost.key",
			StoragePath:         "/tmp/gofer.db",
			StorageResultsLimit: 200,
		},

		Extensions: &Extensions{
			InstallBaseExtensions: true,
			StopTimeout:           time.Minute * 5,
			StopTimeoutHCL:        "5m",
			TLSCertPath:           "./test",
			TLSKeyPath:            "./localhost.key",
		},
	}

	diff := cmp.Diff(expected, hclconf)
	if diff != "" {
		t.Errorf("result is different than expected(-want +got):\n%s", diff)
	}
}
