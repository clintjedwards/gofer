package config

import (
	"fmt"
	"os"
	"sort"
	"strings"
	"time"

	"github.com/fatih/structs"
	"github.com/knadh/koanf/parsers/hcl"
	"github.com/knadh/koanf/providers/env"
	"github.com/knadh/koanf/providers/file"
	"github.com/knadh/koanf/v2"
)

// API defines config settings for the gofer server
type API struct {
	// Controls the ability to extension runs. This setting can be toggled while the server is running.
	IgnorePipelineRunEvents bool `koanf:"ignore_pipeline_run_events"`

	// The limit automatically imposed if the pipeline does not define a limit. 0 is unlimited.
	RunParallelismLimit int `koanf:"run_parallelism_limit"`

	// How many total versions of an individual pipelines to keep.
	// The oldest version of a pipeline over this limit gets deleted.
	// 0 means don't delete versions.
	PipelineVersionLimit int `koanf:"pipeline_version_limit"`

	// Controls how long Gofer will hold onto events before discarding them. This is important factor in disk space
	// and memory footprint.
	//
	// Example: Rough math on a 5,000 pipeline Gofer instance with a full 6 months of retention
	//  puts the memory and storage footprint at about 9GB.
	EventLogRetention time.Duration `koanf:"event_log_retention"`

	// How often the background process for pruning events should run.
	EventPruneInterval time.Duration `koanf:"event_prune_interval"`

	// Log level affects the entire application's logs including launched extensions.
	LogLevel string `koanf:"log_level"`

	// The total amount of runs before logs of the oldest log starts being deleted.
	TaskRunLogExpiry int `koanf:"task_run_log_expiry"`

	// Directory to store task run log files.
	TaskRunLogsDir string `koanf:"task_run_logs_dir"`

	// TaskRunStopTimeout controls the time the scheduler will wait for a normal user container(non-extension containers)
	// to stop. When the timeout is reached the container will be forcefully terminated.
	// You can use a negative duration("-1s") to convey that no timeout should be specified and the scheduler
	// should wait however long it takes the container to respond to the terminate signal.
	// This is usually passed to the scheduler when a request to cancel a task run is being made.
	TaskRunStopTimeout time.Duration `koanf:"task_run_stop_timeout"`

	Development       *Development       `koanf:"development"`
	Extensions        *Extensions        `koanf:"extensions"`
	ExternalEventsAPI *ExternalEventsAPI `koanf:"external_events_api"`
	ObjectStore       *ObjectStore       `koanf:"object_store"`
	Scheduler         *Scheduler         `koanf:"scheduler"`
	SecretStore       *SecretStore       `koanf:"secret_store"`
	Server            *Server            `koanf:"server"`
}

func DefaultAPIConfig() *API {
	return &API{
		IgnorePipelineRunEvents: false,
		RunParallelismLimit:     200,
		PipelineVersionLimit:    5,
		EventLogRetention:       mustParseDuration("4380h"), // 4380 hours is roughly 6 months.
		EventPruneInterval:      mustParseDuration("3h"),
		LogLevel:                "info",
		TaskRunLogExpiry:        50,
		TaskRunLogsDir:          "/tmp",
		TaskRunStopTimeout:      mustParseDuration("5m"),
		Development:             DefaultDevelopmentConfig(),
		Extensions:              DefaultExtensionsConfig(),
		ExternalEventsAPI:       DefaultExternalEventsAPIConfig(),
		ObjectStore:             DefaultObjectStoreConfig(),
		Scheduler:               DefaultSchedulerConfig(),
		SecretStore:             DefaultSecretStoreConfig(),
		Server:                  DefaultServerConfig(),
	}
}

type Development struct {
	PrettyLogging   bool `koanf:"pretty_logging"`
	BypassAuth      bool `koanf:"bypass_auth"`
	UseLocalhostTLS bool `koanf:"use_localhost_tls"`

	// Use a pre-filled insecure encryption key.
	DefaultEncryption bool `koanf:"default_encryption"`

	// Pass the "skip_tls_verify" environment variable to extensions
	// so that they can talk to Gofer without verifying the cert.
	ExtensionSkipTLSVerify bool `koanf:"extension_skip_tls_verify"`
}

func DefaultDevelopmentConfig() *Development {
	return &Development{
		PrettyLogging:          false,
		BypassAuth:             false,
		UseLocalhostTLS:        false,
		DefaultEncryption:      false,
		ExtensionSkipTLSVerify: false,
	}
}

func FullDevelopmentConfig() *Development {
	return &Development{
		PrettyLogging:          true,
		BypassAuth:             true,
		UseLocalhostTLS:        true,
		DefaultEncryption:      true,
		ExtensionSkipTLSVerify: true,
	}
}

// Server represents lower level HTTP/GRPC server settings.
type Server struct {
	// URL where the Gofer server is located. Shared with entities that need to talk to the Gofer API.
	Address string `koanf:"address"`

	// URL for the server to bind to. Ex: localhost:8080
	Host string `koanf:"host"`

	// How long the GRPC service should wait on in-progress connections before hard closing everything out.
	ShutdownTimeout time.Duration `koanf:"shutdown_timeout"`

	// Path to Gofer's sqlite database.
	StoragePath string `koanf:"storage_path"`

	// The total amount of results the database will attempt to pass back when a limit is not explicitly given.
	StorageResultsLimit int `koanf:"storage_results_limit"`

	TLSCertPath string `koanf:"tls_cert_path"`
	TLSKeyPath  string `koanf:"tls_key_path"`
}

// DefaultServerConfig returns a pre-populated configuration struct that is used as the base for super imposing user configuration
// settings.
func DefaultServerConfig() *Server {
	return &Server{
		Address:             "172.17.0.1:8080",
		Host:                "0.0.0.0:8080",
		ShutdownTimeout:     mustParseDuration("15s"),
		StoragePath:         "/tmp/gofer.db",
		StorageResultsLimit: 200,
	}
}

// Extensions represents the configuration for Gofer Extensions. Extensions are used to generate events in which pipelines
// should run.
type Extensions struct {
	// InstallBaseExtensions attempts to automatically install the cron and interval extensions on first startup.
	InstallBaseExtensions bool `koanf:"install_base_extensions"`

	// StopTimeout controls the time the scheduler will wait for a extension container to stop. After this period
	// Gofer will attempt to force stop the extension container.
	StopTimeout time.Duration `koanf:"stop_timeout"`

	// TLSCertPath is the file path of the extension TLS certificate.
	TLSCertPath string `koanf:"tls_cert_path"`

	// TLSKeyPath is the file path of the extension TLS key.
	TLSKeyPath string `koanf:"tls_key_path"`
}

func DefaultExtensionsConfig() *Extensions {
	return &Extensions{
		InstallBaseExtensions: true,
		StopTimeout:           mustParseDuration("5m"),
	}
}

// Frontend represents configuration for frontend basecoat
type Frontend struct {
	Enable bool `koanf:"enable"`
}

// ExternalEventsAPI controls how the settings around the HTTP service that handles external extension events.
type ExternalEventsAPI struct {
	Enable bool `koanf:"enable"`

	// URL for the server to bind to. Ex: localhost:8080
	Host string `koanf:"host"`
}

func DefaultExternalEventsAPIConfig() *ExternalEventsAPI {
	return &ExternalEventsAPI{
		Enable: true,
		Host:   "localhost:8081",
	}
}

// Get the final configuration for the server.
// This involves correctly finding and ordering different possible paths for the configuration file:
//
//  1. The function is intended to be called with paths gleaned from the -config flag in the cli.
//  2. If the user does not use the -config path of the path does not exist,
//     then we default to a few hard coded config path locations.
//  3. Then try to see if the user has set an envvar for the config file, which overrides
//     all previous config file paths.
//  4. Finally, whatever configuration file path is found first is the processed.
//
// Whether or not we use the configuration file we then search the environment for all environment variables:
//   - Environment variables are loaded after the config file and therefore overwrite any conflicting keys.
//   - All configuration that goes into a configuration file can also be used as an environment variable.
func InitAPIConfig(userDefinedPath string, loadDefaults, validate, devMode bool) (*API, error) {
	var config *API

	// First we initiate the default values for the config.
	if loadDefaults {
		config = DefaultAPIConfig()
	}

	if devMode {
		config.Development = FullDevelopmentConfig()
	}

	possibleConfigPaths := []string{userDefinedPath, "/etc/gofer/gofer.hcl"}

	path := searchFilePaths(possibleConfigPaths...)

	// envVars top all other entries so if its not empty we just insert it over the current path
	// regardless of if we found one.
	envPath := os.Getenv("GOFER_CONFIG_PATH")
	if envPath != "" {
		path = envPath
	}

	configParser := koanf.New(".")

	if path != "" {
		err := configParser.Load(file.Provider(path), hcl.Parser(true))
		if err != nil {
			return nil, err
		}
	}

	err := configParser.Load(env.Provider("GOFER_", "__", func(s string) string {
		newStr := strings.TrimPrefix(s, "GOFER_")
		newStr = strings.ToLower(newStr)
		return newStr
	}), nil)
	if err != nil {
		return nil, err
	}

	err = configParser.Unmarshal("", &config)
	if err != nil {
		return nil, err
	}

	if validate {
		err = config.validate()
		if err != nil {
			return nil, err
		}
	}

	return config, nil
}

func (c *API) validate() error {
	if c.SecretStore != nil && c.SecretStore.Sqlite != nil {

		if len(c.SecretStore.Sqlite.EncryptionKey) != 32 {
			return fmt.Errorf("encryption_key must be a 32 character random string")
		}

		if !c.Development.DefaultEncryption && c.SecretStore.Sqlite.EncryptionKey == "changemechangemechangemechangeme" {
			return fmt.Errorf("encryption_key cannot be left as default; must be changed to a 32 character random string")
		}
	}

	return nil
}

func GetAPIEnvVars() []string {
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

	vars := getEnvVarsFromStruct("GOFER_", fields)
	sort.Strings(vars)
	return vars
}
