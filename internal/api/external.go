package api

import (
	"context"
	"net/http"
	"os"
	"os/signal"
	"syscall"
	"time"

	"github.com/clintjedwards/gofer/internal/config"
	"github.com/gorilla/handlers"
	"github.com/gorilla/mux"
	"github.com/rs/zerolog/log"
)

// StartExternalEventsService starts the external events http service, which is used to pass external events (like a github webhook)
// to extensions.
func StartExternalEventsService(config *config.API, api *API) {
	router := mux.NewRouter()

	router.Handle("/external/{extension}", handlers.MethodHandler{
		"POST": http.HandlerFunc(api.externalEventsHandler),
	})

	tlsConfig, err := api.generateTLSConfig(api.config.Server.TLSCertPath, api.config.Server.TLSKeyPath)
	if err != nil {
		log.Fatal().Err(err).Msg("could not get proper TLS config")
	}

	httpServer := http.Server{
		Addr:         config.ExternalEventsAPI.Host,
		Handler:      router,
		WriteTimeout: 15 * time.Second,
		ReadTimeout:  15 * time.Second,
		IdleTimeout:  15 * time.Second,
		TLSConfig:    tlsConfig,
	}

	// Run our server in a goroutine and listen for signals that indicate graceful shutdown
	go func() {
		if err := httpServer.ListenAndServeTLS(config.Server.TLSCertPath, config.Server.TLSKeyPath); err != nil && err != http.ErrServerClosed {
			log.Fatal().Err(err).Msg("server exited abnormally")
		}
	}()
	log.Info().Str("url", config.ExternalEventsAPI.Host).Msg("started gofer external events http service")

	c := make(chan os.Signal, 1)
	signal.Notify(c, syscall.SIGTERM, syscall.SIGINT)
	<-c

	// Doesn't block if no connections, otherwise will wait until the timeout deadline or connections to finish,
	// whichever comes first.
	ctx, cancel := context.WithTimeout(context.Background(), config.Server.ShutdownTimeout) // shutdown gracefully
	defer cancel()

	err = httpServer.Shutdown(ctx)
	if err != nil {
		log.Error().Err(err).Msg("could not shutdown server in timeout specified")
		return
	}

	log.Info().Msg("external events service exited gracefully")
}
