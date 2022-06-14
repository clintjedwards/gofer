package sdk

import (
	"fmt"
	"os"
	"strings"

	sdkProto "github.com/clintjedwards/gofer/gofer_sdk/go/proto"
	"github.com/kelseyhightower/envconfig"
	"github.com/rs/zerolog"
	"github.com/rs/zerolog/log"
)

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

func setupLogging(triggerKind, loglevel string) {
	zerolog.TimeFieldFormat = zerolog.TimeFormatUnix
	log.Logger = log.With().Caller().Logger()
	log.Logger = log.With().Str("trigger", triggerKind).Logger()
	zerolog.SetGlobalLevel(parseLogLevel(loglevel))
}

func parseLogLevel(loglevel string) zerolog.Level {
	switch loglevel {
	case "debug":
		return zerolog.DebugLevel
	case "info":
		return zerolog.InfoLevel
	case "warn":
		return zerolog.WarnLevel
	case "error":
		return zerolog.ErrorLevel
	case "fatal":
		return zerolog.FatalLevel
	case "panic":
		return zerolog.PanicLevel
	default:
		log.Error().Msgf("loglevel %s not recognized; defaulting to debug", loglevel)
		return zerolog.DebugLevel
	}
}

// GetConfig is a convienance function that returns trigger/notifier config values from the environment.
// It simply puts the needed config in the correct format to be retrieved from the environment
// so the caller doesn't have to.
func GetConfig(name string) string {
	kind := os.Getenv("GOFER_TRIGGER_KIND")
	return os.Getenv(fmt.Sprintf("GOFER_TRIGGER_%s_%s", strings.ToUpper(kind), strings.ToUpper(name)))
}

// InfoResponse is a convienance function for the Info interface function response.
func InfoResponse(documentationLink string) (*sdkProto.InfoResponse, error) {
	return &sdkProto.InfoResponse{
		Kind:          os.Getenv("GOFER_TRIGGER_KIND"),
		Documentation: documentationLink,
	}, nil
}

// NewTrigger is used as the final step in establishing a trigger. It should be the final call in a trigger's main func.
//
// It takes two parameters:
// 1) The concrete service implementation which is turned into a GRPC service in order to handle pipeline trigger events.
// 2) A installer function which is called upon when a user wants to install this particular trigger.
// More documentation for the implementation is coming soon: TODO(clintjedwards):
func NewTrigger(service TriggerServerInterface, installer func()) {
	if len(os.Args) != 2 {
		log.Fatal().Msg("Usage: ./trigger <server|installer>")
	}

	switch os.Args[1] {
	case "server":
		newTriggerServer(service)
	case "installer":
		installer()
	default:
		log.Fatal().Msg("Usage: ./trigger <server|installer>")
	}
}
