package sdk

import "github.com/kelseyhightower/envconfig"

type Config struct {
	// Key is the auth key passed by the main gofer application to prevent other
	// actors from attempting to communicate with the triggers.
	Key  string `required:"true" json:"-"`
	Kind string `required:"true"`
	// Possible values "debug", "info", "warn", "error", "fatal", "panic"
	LogLevel string `split_words:"true" default:"info"`
	// Contains the raw bytes for a TLS cert used by the trigger to authenticate clients.
	TLSCert string `split_words:"true" required:"true" json:"-"`
	TLSKey  string `split_words:"true" required:"true" json:"-"`
	Host    string `default:"0.0.0.0:8080"`
}

func initConfig() (*Config, error) {
	config := Config{}
	err := envconfig.Process("gofer_trigger", &config)
	if err != nil {
		return nil, err
	}

	return &config, nil
}
