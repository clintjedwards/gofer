package config

import (
	"encoding/json"
	"fmt"
	"os"
	"time"

	"github.com/hashicorp/hcl/v2/hclsimple"
	"github.com/kelseyhightower/envconfig"
)

// API defines config settings for the gofer server
type API struct {
	// Controls the ability to trigger runs. This setting can be toggled while the server is running.
	AcceptEventsOnStartup bool `split_words:"true" hcl:"accept_events_on_startup,optional"`

	// Controls how large the buffer space for the event loop channel is.
	EventLoopChannelSize int64 `split_words:"true" hcl:"event_loop_channel_size,optional"`

	// EncryptionKey is used to encrypt secret values into the database.
	EncryptionKey string `split_words:"true" hcl:"encryption_key,optional"`

	// URL for the server to bind to. Ex: localhost:8080
	Host string `hcl:"host,optional"`

	// Log level affects the entire application's logs including launched triggers.
	LogLevel string `split_words:"true" hcl:"log_level,optional"`

	// The total amount of runs before logs of the oldest log starts being deleted.
	RunLogExpiry int `split_words:"true" hcl:"run_log_expiry,optional"`

	// Directory to store task run log files.
	TaskRunLogsDir string `split_words:"true" hcl:"task_run_logs_dir,optional"`

	// TaskRunStopTimeout controls the time the scheduler will wait for a normal user container(non-trigger containers)
	// to stop. When the timeout is reached the container will be forcefully terminated.
	// You can use a negative duration("-1s") to convey that no timeout should be specified and the scheduler
	// should wait however long it takes the container to respond to the terminate signal.
	// This is usually passed to the scheduler when a request to cancel a task run is being made.
	TaskRunStopTimeout time.Duration `split_words:"true"`

	// TaskRunStopTimeoutHCL is the HCL compatible counter part to TaskRunStopTimeout. It allows the parsing of a string
	// to a time.Duration since HCL does not support parsing directly into a time.Duration.
	TaskRunStopTimeoutHCL string `ignored:"true" hcl:"task_run_stop_timeout,optional"`

	ExternalEventsAPI *ExternalEventsAPI `split_words:"true" hcl:"external_events_api,block"`
	Database          *Database          `hcl:"database,block"`
	ObjectStore       *ObjectStore       `hcl:"object_store,block"`
	Scheduler         *Scheduler         `hcl:"scheduler,block"`
	Server            *Server            `hcl:"server,block"`
	Triggers          *Triggers          `hcl:"triggers,block"`
}

func DefaultAPIConfig() *API {
	return &API{
		AcceptEventsOnStartup: true,
		EventLoopChannelSize:  100,
		Host:                  "localhost:8080",
		LogLevel:              "debug",
		EncryptionKey:         "default",
		RunLogExpiry:          20,
		TaskRunLogsDir:        "/tmp",
		TaskRunStopTimeout:    mustParseDuration("5m"),

		ExternalEventsAPI: DefaultExternalEventsAPIConfig(),
		Database:          DefaultDatabaseConfig(),
		ObjectStore:       DefaultObjectStoreConfig(),
		Scheduler:         DefaultSchedulerConfig(),
		Server:            DefaultServerConfig(),
		Triggers:          DefaultTriggersConfig(),
	}
}

// Server respresents lower level HTTP/GRPC server settings.
type Server struct {
	// DevMode turns on humanized debug messages, extra debug logging for the webserver and other
	// convenient features for development. Usually turned on along side LogLevel=debug.
	DevMode bool `hcl:"dev_mode,optional"`

	// How long the GRPC service should wait on in-progress connections before hard closing everything out.
	ShutdownTimeout time.Duration `split_words:"true"`

	// ShutdownTimeoutHCL is the HCL compatible counter part to ShutdownTimeout. It allows the parsing of a string
	// to a time.Duration since HCL does not support parsing directly into a time.Duration.
	ShutdownTimeoutHCL string `ignored:"true" hcl:"shutdown_timeout,optional"`

	TLSCertPath string `split_words:"true" hcl:"tls_cert_path,optional"`
	TLSKeyPath  string `split_words:"true" hcl:"tls_key_path,optional"`

	// Temporary storage for downloaded pipeline configs.
	TmpDir string `split_words:"true" hcl:"tmp_dir,optional"`
}

// DefaultServerConfig returns a pre-populated configuration struct that is used as the base for super imposing user configuration
// settings.
func DefaultServerConfig() *Server {
	return &Server{
		DevMode:         true,
		ShutdownTimeout: mustParseDuration("15s"),
		TmpDir:          "/tmp",
	}
}

// Triggers represents the configuration for Gofer Triggers. Triggers are used to generate events in which pipelines
// should run.
type Triggers struct {
	// StopTimeout controls the time the scheduler will wait for a trigger container to stop. After this period
	// Gofer will attempt to force stop the trigger container.
	StopTimeout time.Duration `split_words:"true"`

	// StopTimeoutHCL is the HCL compatible counter part to TriggerStopTimeout. It allows the parsing of a string
	// to a time.Duration since HCL does not support parsing directly into a time.Duration.
	StopTimeoutHCL string `ignored:"true" hcl:"stop_timeout,optional"`

	// HealthcheckInterval defines the period of time between attempted GRPC connections to all triggers. Triggers
	// are healthchecked to ensure proper operation.
	HealthcheckInterval time.Duration `split_words:"true"`

	// HealthcheckInternalHCL is the HCL compatible counter part to TriggerHealthcheck. It allows the parsing of a string
	// to a time.Duration since HCL does not support parsing directly into a time.Duration.
	HealthcheckIntervalHCL string `ignored:"true" hcl:"healthcheck_interval,optional"`

	// TLSCertPath is the file path of the trigger TLS certificate.
	TLSCertPath string `split_words:"true" hcl:"tls_cert_path,optional"`

	// TLSKeyPath is the file path of the trigger TLS key.
	TLSKeyPath string `split_words:"true" hcl:"tls_key_path,optional"`

	// RegisteredTriggers represents the triggers that Gofer will attempt to startup with.
	RegisteredTriggers RegisteredTriggers `split_words:"true" hcl:"registered_triggers,block"`
}

func DefaultTriggersConfig() *Triggers {
	return &Triggers{
		StopTimeout:         mustParseDuration("5m"),
		HealthcheckInterval: mustParseDuration("30s"),
		RegisteredTriggers:  []Trigger{},
	}
}

// Trigger represents the settings for all triggers within Gofer.
type Trigger struct {
	// The name for a trigger this should be alphanumerical and can't contain spaces.
	Kind string `json:"kind" hcl:"kind,label"`

	// The docker repository and image name of the trigger: Ex. docker.io/library/hello-world:latest
	Image string `json:"image" hcl:"image"`

	// The user id for the docker repository; if needed.
	User string `json:"user" hcl:"user,optional"`

	// The password for the docker repository; if needed.
	Pass string `json:"pass" hcl:"pass,optional"`

	// Environment variables to pass to the trigger container. This is used to pass runtime settings to the container.
	EnvVars map[string]string `json:"env_vars" hcl:"env_vars,optional"`

	// Secrets to pass to the trigger container. The way secrets are passed depends on the scheduler.
	Secrets map[string]string `json:"secrets" hcl:"secrets,optional"`
}

// RegisteredTrigger represents the list of triggers that Gofer will attempt to startup with and use.
type RegisteredTriggers []Trigger

// Set is a method that implements the ability for envconfig to unfurl a trigger mentioned as an environment variable.
// Basically the trigger is just wrapped up as a json blurb and set will unwrap it into the proper struct.
func (t *RegisteredTriggers) Set(value string) error {
	triggers := []Trigger{}

	err := json.Unmarshal([]byte(value), &triggers)
	if err != nil {
		return err
	}

	*t = RegisteredTriggers(triggers)
	return nil
}

// Frontend represents configuration for frontend basecoat
type Frontend struct {
	Enable bool `hcl:"enable,optional"`
}

// ExternalEventsAPI controls how the settings around the HTTP service that handles external trigger events.
type ExternalEventsAPI struct {
	Enable bool `hcl:"enable,optional"`

	// URL for the server to bind to. Ex: localhost:8080
	Host string `hcl:"host,optional"`
}

func DefaultExternalEventsAPIConfig() *ExternalEventsAPI {
	return &ExternalEventsAPI{
		Enable: true,
		Host:   "localhost:8081",
	}
}

func defaultTriggers() []Trigger {
	return []Trigger{
		{Kind: "cron", Image: "ghcr.io/clintjedwards/gofer/trigger_cron:latest"},
		{Kind: "interval", Image: "ghcr.io/clintjedwards/gofer/trigger_interval:latest", EnvVars: map[string]string{
			"GOFER_TRIGGER_INTERVAL_MIN_DURATION": "5m",
		}},
	}
}

// FromEnv parses environment variables into the config object based on envconfig name
func (c *API) FromEnv() error {
	err := envconfig.Process("gofer", c)
	if err != nil {
		return err
	}

	return nil
}

// FromBytes attempts to parse a given HCL configuration.
func (c *API) FromBytes(content []byte) error {
	err := hclsimple.Decode("config.hcl", content, nil, c)
	if err != nil {
		return err
	}

	c.convertDurationFromHCL()

	return nil
}

func (c *API) FromFile(path string) error {
	err := hclsimple.DecodeFile(path, nil, c)
	if err != nil {
		return err
	}

	c.convertDurationFromHCL()

	return nil
}

// convertDurationFromHCL attempts to move the string value of a duration written in HCL to
// the real time.Duration type. This is needed due to advanced types like time.Duration being not handled particularly
// well during HCL parsing: https://github.com/hashicorp/hcl/issues/202
func (c *API) convertDurationFromHCL() {
	if c.Server != nil && c.Server.ShutdownTimeoutHCL != "" {
		c.Server.ShutdownTimeout = mustParseDuration(c.Server.ShutdownTimeoutHCL)
	}

	if c != nil && c.Triggers.HealthcheckIntervalHCL != "" {
		c.Triggers.HealthcheckInterval = mustParseDuration(c.Triggers.HealthcheckIntervalHCL)
	}

	if c != nil && c.TaskRunStopTimeoutHCL != "" {
		c.TaskRunStopTimeout = mustParseDuration(c.TaskRunStopTimeoutHCL)
	}

	if c != nil && c.Triggers.StopTimeoutHCL != "" {
		c.Triggers.StopTimeout = mustParseDuration(c.Triggers.StopTimeoutHCL)
	}

	if c.Scheduler != nil && c.Scheduler.Docker.PruneIntervalHCL != "" {
		c.Scheduler.Docker.PruneInterval = mustParseDuration(c.Scheduler.Docker.PruneIntervalHCL)
	}
}

// Get the final configuration for the server.
// This involves correctly finding and ordering different possible paths for the configuration file.
//
// 1) The function is intended to be called with paths gleaned from the -config flag
// 2) Then combine that with possible other config locations that the user might store a config file.
// 3) Then try to see if the user has set an envvar for the config file, which overrides
// all previous config file paths.
// 4) Finally, pass back whatever is deemed the final config path from that process.
//
// We then use that path data to find the config file and read it in via HCL parsers. Once that is finished
// we then take any configuration from the environment and superimpose that on top of the final config struct.
func InitAPIConfig(userDefinedPath string) (*API, error) {
	// First we initiate the default values for the config.
	config := DefaultAPIConfig()

	possibleConfigPaths := []string{userDefinedPath, "/etc/gofer/gofer.hcl"}

	path := searchFilePaths(possibleConfigPaths...)

	// envVars top all other entries so if its not empty we just insert it over the current path
	// regardless of if we found one.
	envPath := os.Getenv("GOFER_CONFIG_PATH")
	if envPath != "" {
		path = envPath
	}

	if path != "" {
		err := config.FromFile(path)
		if err != nil {
			return nil, err
		}
	}

	err := config.FromEnv()
	if err != nil {
		return nil, err
	}

	// Always append default triggers
	config.Triggers.RegisteredTriggers = append(config.Triggers.RegisteredTriggers, defaultTriggers()...)

	return config, nil
}

func PrintAPIEnvs() error {
	var config API
	err := envconfig.Usage("gofer", &config)
	if err != nil {
		return err
	}
	fmt.Println("GOFER_CONFIG_PATH")

	return nil
}
