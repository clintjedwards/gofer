// package sdk contains functions to aid in the building of trigger plugins.
//
// The goal is to provide a light abstraction over the nitty gritty of implementing GRPC interactions
// so the caller has a clear interface to implement and is provided with common patterns automatically.
//
// You can find more about triggers and implementing new triggers here: https://clintjedwards.com/gofer/docs/triggers/overview
package sdk

import (
	"os"

	"github.com/rs/zerolog"
	"github.com/rs/zerolog/log"
)

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
