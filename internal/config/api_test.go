package config

import (
	"encoding/json"
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
		AcceptEventsOnStartup: true,
		EventLoopChannelSize:  100,
		Host:                  "localhost:8080",
		LogLevel:              "info",
		RunLogExpiry:          20,
		TaskRunLogsDir:        "/tmp",
		TaskRunStopTimeout:    time.Minute * 5,
		TaskRunStopTimeoutHCL: "5m",

		ExternalEventsAPI: &ExternalEventsAPI{
			Enable: true,
			Host:   "localhost:8081",
		},

		Database: &Database{
			Engine:          "bolt",
			MaxResultsLimit: 100,
			BoltDB: &BoltDB{
				Path: "/tmp/gofer.db",
			},
		},

		ObjectStore: &ObjectStore{
			Engine: "bolt",
			BoltDB: &BoltDB{
				Path: "/tmp/gofer-os.db",
			},
			PipelineObjectLimit: 10,
			RunObjectExpiry:     20,
		},

		SecretStore: &SecretStore{
			Engine: "bolt",
			BoltDB: &BoltDBSecret{
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
			DevMode:            false,
			ShutdownTimeout:    time.Second * 15,
			ShutdownTimeoutHCL: "15s",
			TLSCertPath:        "./localhost.crt",
			TLSKeyPath:         "./localhost.key",
			TmpDir:             "/tmp",
		},

		Triggers: &Triggers{
			StopTimeout:            time.Minute * 5,
			StopTimeoutHCL:         "5m",
			HealthcheckInterval:    time.Second * 30,
			HealthcheckIntervalHCL: "30s",
			TLSCertPath:            "./localhost.crt",
			TLSKeyPath:             "./localhost.key",
			RegisteredTriggers: []Trigger{
				{
					Kind:  "cron",
					Image: "ghcr.io/clintjedwards/gofer/trigger_cron:latest",
				},
				{
					Kind:  "interval",
					Image: "ghcr.io/clintjedwards/gofer/trigger_interval:latest",
				},
			},
		},
	}

	diff := cmp.Diff(expected, hclconf)
	if diff != "" {
		t.Errorf("result is different than expected(-want +got):\n%s", diff)
	}
}

func TestAPISampleOverwriteWithEnvs(t *testing.T) {
	_ = os.Setenv("GOFER_ACCEPT_EVENTS_ON_STARTUP", "false")
	_ = os.Setenv("GOFER_EXTERNAL_EVENTS_API_ENABLE", "false")
	_ = os.Setenv("GOFER_DATABASE_MAX_RESULTS_LIMIT", "1000")
	_ = os.Setenv("GOFER_OBJECTSTORE_RUN_OBJECT_EXPIRY", "1000")
	_ = os.Setenv("GOFER_SCHEDULER_DOCKER_PRUNE", "false")
	_ = os.Setenv("GOFER_SERVER_TLS_CERT_PATH", "./test")
	_ = os.Setenv("GOFER_TRIGGERS_TLS_CERT_PATH", "./test")
	defer os.Unsetenv("GOFER_ACCEPT_EVENTS_ON_STARTUP")
	defer os.Unsetenv("GOFER_EXTERNAL_EVENTS_API_ENABLE")
	defer os.Unsetenv("GOFER_DATABASE_MAX_RESULTS_LIMIT")
	defer os.Unsetenv("GOFER_OBJECTSTORE_RUN_OBJECT_EXPIRY")
	defer os.Unsetenv("GOFER_SCHEDULER_DOCKER_PRUNE")
	defer os.Unsetenv("GOFER_SERVER_TLS_CERT_PATH")
	defer os.Unsetenv("GOFER_TRIGGERS_TLS_CERT_PATH")

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
		AcceptEventsOnStartup: false,
		EventLoopChannelSize:  100,
		Host:                  "localhost:8080",
		LogLevel:              "info",
		RunLogExpiry:          20,
		TaskRunLogsDir:        "/tmp",
		TaskRunStopTimeout:    time.Minute * 5,
		TaskRunStopTimeoutHCL: "5m",

		ExternalEventsAPI: &ExternalEventsAPI{
			Enable: false,
			Host:   "localhost:8081",
		},

		Database: &Database{
			Engine:          "bolt",
			MaxResultsLimit: 1000,
			BoltDB: &BoltDB{
				Path: "/tmp/gofer.db",
			},
		},

		ObjectStore: &ObjectStore{
			Engine: "bolt",
			BoltDB: &BoltDB{
				Path: "/tmp/gofer-os.db",
			},
			PipelineObjectLimit: 10,
			RunObjectExpiry:     1000,
		},

		SecretStore: &SecretStore{
			Engine: "bolt",
			BoltDB: &BoltDBSecret{
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
			DevMode:            false,
			ShutdownTimeout:    time.Second * 15,
			ShutdownTimeoutHCL: "15s",
			TLSCertPath:        "./test",
			TLSKeyPath:         "./localhost.key",
			TmpDir:             "/tmp",
		},

		Triggers: &Triggers{
			StopTimeout:            time.Minute * 5,
			StopTimeoutHCL:         "5m",
			HealthcheckInterval:    time.Second * 30,
			HealthcheckIntervalHCL: "30s",
			TLSCertPath:            "./test",
			TLSKeyPath:             "./localhost.key",
			RegisteredTriggers: []Trigger{
				{
					Kind:  "cron",
					Image: "ghcr.io/clintjedwards/gofer/trigger_cron:latest",
				},
				{
					Kind:  "interval",
					Image: "ghcr.io/clintjedwards/gofer/trigger_interval:latest",
				},
			},
		},
	}

	diff := cmp.Diff(expected, hclconf)
	if diff != "" {
		t.Errorf("result is different than expected(-want +got):\n%s", diff)
	}
}

func TestSetTriggersViaEnv(t *testing.T) {
	trigger := `[{"kind": "cron","image": "docker.io/library/hello-world:latest","user": "test","pass": "pass",`
	trigger += `"env_vars": {"envvar1": "hello"},"secrets": {"secretone": "weow"}}]`

	// First check that the above is even valid json.
	err := json.Unmarshal([]byte(trigger), &[]map[string]interface{}{})
	if err != nil {
		t.Fatal(err)
	}

	_ = os.Setenv("GOFER_TRIGGERS_REGISTERED_TRIGGERS", string(trigger))
	defer os.Unsetenv("GOFER_TRIGGERS_REGISTERED_TRIGGERS")

	hclconf := API{}
	err = hclconf.FromEnv()
	if err != nil {
		t.Fatal(err)
	}

	expected := API{
		ExternalEventsAPI: &ExternalEventsAPI{},
		Database: &Database{
			BoltDB: &BoltDB{},
		},
		ObjectStore: &ObjectStore{
			BoltDB: &BoltDB{},
		},
		SecretStore: &SecretStore{
			BoltDB: &BoltDBSecret{},
		},
		Scheduler: &Scheduler{
			Docker: &Docker{},
		},
		Server: &Server{},
		Triggers: &Triggers{
			RegisteredTriggers: []Trigger{
				{
					Kind:  "cron",
					Image: "docker.io/library/hello-world:latest",
					User:  "test",
					Pass:  "pass",
					EnvVars: map[string]string{
						"envvar1": "hello",
					},
					Secrets: map[string]string{
						"secretone": "weow",
					},
				},
			},
		},
	}

	diff := cmp.Diff(expected, hclconf)
	if diff != "" {
		t.Errorf("result is different than expected(-want +got):\n%s", diff)
	}
}
