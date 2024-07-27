package extensions

import (
	"context"
	"crypto/tls"
	"encoding/json"
	"fmt"
	"net/http"
	"os"
	"os/signal"
	"strings"
	"syscall"
	"time"

	"github.com/go-chi/chi/v5/middleware"

	"github.com/kelseyhightower/envconfig"
	"github.com/rs/zerolog"
	"github.com/rs/zerolog/log"
)

// ExtensionServiceInterface provides a light wrapper around the GRPC extension interface. This light wrapper
// provides the caller with a clear interface to implement and allows this package to bake in common
// functionality among all extensions.
type ExtensionServiceInterface interface {
	// A simple healthcheck endpoint used by Gofer to make sure the extension is still in good health and reachable.
	Health(context.Context) *HttpError

	// Returns information specific to the extension.
	Info(context.Context) (*InfoResponse, *HttpError)

	// Allows the extension to print any information relevant to it's execution.
	// This endpoint is freely open so make sure to not include any particularly sensitive information in this
	// endpoint.
	Debug(context.Context) DebugResponse

	// Registers a pipeline with said extension to provide the extension's functionality.
	Subscribe(context.Context, SubscriptionRequest) *HttpError

	// Allows pipelines to remove their extension subscriptions.
	Unsubscribe(context.Context, UnsubscriptionRequest) *HttpError

	// Shutdown tells the extension to cleanup and gracefully shutdown. If a extension
	// does not shutdown in a time defined by the Gofer API the extension will
	// instead be Force shutdown(SIGKILL). This is to say that all extensions should
	// lean toward quick cleanups and shutdowns.
	Shutdown(context.Context)

	// ExternalEvent are json blobs of Gofer's /events endpoint. Normally webhooks.
	ExternalEvent(context.Context, ExternalEventRequest) *HttpError
}

// Error Error information from a response.
type HttpError struct {
	StatusCode int    `json:"status_code"`
	Message    string `json:"message"`
}

type ErrorResponse struct {
	Message   string `json:"message"`
	RequestID string `json:"request_id"`
}

func handleError(w http.ResponseWriter, _ *http.Request, message string, statusCode int, err error) {
	w.WriteHeader(statusCode)
	w.Header().Set("Content-Type", "application/json")

	response := ErrorResponse{
		Message:   fmt.Sprintf("%s: %s; %+v", http.StatusText(statusCode), message, err),
		RequestID: "",
	}
	err = json.NewEncoder(w).Encode(response)
	if err != nil {
		log.Error().Err(err).Msg("Could not serialize error")
	}
}

func handleResponse(w http.ResponseWriter, _ *http.Request, data any, statusCode int) {
	w.WriteHeader(statusCode)
	w.Header().Set("Content-Type", "application/json")

	if data != nil {
		err := json.NewEncoder(w).Encode(data)
		if err != nil {
			log.Error().Err(err).Msg("Could not serialize response")
		}
	}
}

type extensionWrapper struct {
	authKey     string
	extension   ExtensionServiceInterface
	extensionID string
}

func (e *extensionWrapper) healthHandler(w http.ResponseWriter, r *http.Request) {
	ctx := r.Context()
	err := e.extension.Health(ctx)
	if err != nil {
		handleError(w, r, err.Message, err.StatusCode, nil)
		return
	}

	handleResponse(w, r, nil, http.StatusNoContent)
}

func (e *extensionWrapper) debugHandler(w http.ResponseWriter, r *http.Request) {
	ctx := r.Context()
	resp := e.extension.Debug(ctx)

	handleResponse(w, r, resp.Info, http.StatusOK)
}

func (e *extensionWrapper) infoHandler(w http.ResponseWriter, r *http.Request) {
	ctx := r.Context()
	resp, err := e.extension.Info(ctx)
	if err != nil {
		handleError(w, r, err.Message, err.StatusCode, nil)
		return
	}

	resp.ExtensionId = e.extensionID

	handleResponse(w, r, resp, http.StatusOK)
}

func (e *extensionWrapper) subscribeHandler(w http.ResponseWriter, r *http.Request) {
	ctx := r.Context()

	var request SubscriptionRequest

	err := json.NewDecoder(r.Body).Decode(&request)
	if err != nil {
		handleError(w, r, "Could not parse request body", http.StatusBadRequest, err)
		return
	}

	innerErr := e.extension.Subscribe(ctx, request)
	if innerErr != nil {
		handleError(w, r, innerErr.Message, innerErr.StatusCode, nil)
		return
	}

	handleResponse(w, r, nil, http.StatusNoContent)
}

func (e *extensionWrapper) unsubscribeHandler(w http.ResponseWriter, r *http.Request) {
	ctx := r.Context()

	var request UnsubscriptionRequest

	err := json.NewDecoder(r.Body).Decode(&request)
	if err != nil {
		handleError(w, r, "Could not parse request body", http.StatusBadRequest, err)
		return
	}

	innerErr := e.extension.Unsubscribe(ctx, request)
	if innerErr != nil {
		handleError(w, r, innerErr.Message, innerErr.StatusCode, nil)
		return
	}

	handleResponse(w, r, nil, http.StatusCreated)
}

func (e *extensionWrapper) shutdownHandler(w http.ResponseWriter, r *http.Request) {
	ctx := r.Context()
	e.extension.Shutdown(ctx)
	handleResponse(w, r, nil, http.StatusNoContent)
}

func (e *extensionWrapper) externalEventHandler(w http.ResponseWriter, r *http.Request) {
	ctx := r.Context()

	var request ExternalEventRequest

	err := json.NewDecoder(r.Body).Decode(&request)
	if err != nil {
		handleError(w, r, "Could not parse request body", http.StatusBadRequest, err)
		return
	}

	innerErr := e.extension.ExternalEvent(ctx, request)
	if innerErr != nil {
		handleError(w, r, innerErr.Message, innerErr.StatusCode, nil)
		return
	}

	handleResponse(w, r, nil, http.StatusNoContent)
}

// NewExtension starts the provided extension service
func NewExtension(impl ExtensionServiceInterface) {
	config, err := GetExtensionSystemConfig()
	if err != nil {
		log.Fatal().Err(err).Msg("Could not retrieve system configuration via env vars")
	}

	setupLogging(config.ID, config.LogLevel)

	extensionServer := &extensionWrapper{
		authKey:     config.Secret,
		extension:   impl,
		extensionID: config.ID,
	}
	extensionServer.run()
}

// getTLS finds the certificates which are appropriate and
func getTLS() *tls.Config {
	config, _ := GetExtensionSystemConfig()

	serverCert, err := tls.X509KeyPair([]byte(config.TLSCert), []byte(config.TLSKey))
	if err != nil {
		log.Fatal().Err(err).Msg("could not load certificate")
	}

	tlsConfig := &tls.Config{
		Certificates: []tls.Certificate{serverCert},
		ClientAuth:   tls.NoClientCert,
	}

	return tlsConfig
}

// InternalUseOnly - InitRouter creates and adds routes to the internal http router. This function is not meant for
// outside use and mostly just exists to make it easy to generate the OpenAPI spec file..
func InitRouter(e *extensionWrapper) (router *http.ServeMux) {
	router = http.NewServeMux()
	router.HandleFunc("GET /api/health", e.healthHandler)
	router.HandleFunc("GET /api/info", e.infoHandler)
	router.HandleFunc("GET /api/debug", e.debugHandler)
	router.HandleFunc("POST /api/subscribe", e.subscribeHandler)
	router.HandleFunc("DELETE /api/subscribe", e.unsubscribeHandler)
	router.HandleFunc("POST /api/shutdown", e.shutdownHandler)
	router.HandleFunc("POST /api/external-event", e.externalEventHandler)

	return router
}

// run creates a grpc server with all the proper settings; TLS enabled
func (e *extensionWrapper) run() {
	config, err := GetExtensionSystemConfig()
	if err != nil {
		log.Fatal().Err(err).Msg("Could not retrieve system configuration via env vars")
	}

	err = config.validate()
	if err != nil {
		log.Fatal().Err(err).Msg("Could not validate system configuration")
	}

	router := InitRouter(e)

	httpServer := http.Server{
		Addr:         config.BindAddress,
		Handler:      loggingMiddleware(e.authMiddleware(router)),
		WriteTimeout: 15 * time.Second,
		ReadTimeout:  15 * time.Second,
		IdleTimeout:  15 * time.Second,
	}

	if config.UseTLS {
		tlsConfig := getTLS()
		httpServer.TLSConfig = tlsConfig

		// Run our server in a goroutine and listen for signals that indicate graceful shutdown
		go func() {
			if err := httpServer.ListenAndServeTLS("", ""); err != nil && err != http.ErrServerClosed {
				log.Fatal().Err(err).Msg("server exited abnormally")
			}
		}()
	} else {
		// Run our server in a goroutine and listen for signals that indicate graceful shutdown
		go func() {
			if err := httpServer.ListenAndServe(); err != nil && err != http.ErrServerClosed {
				log.Fatal().Err(err).Msg("server exited abnormally")
			}
		}()
	}

	log.Info().Bool("tls_enabled", config.UseTLS).Str("bind_address", config.BindAddress).Msg("started extension http service")

	c := make(chan os.Signal, 1)
	signal.Notify(c, syscall.SIGTERM, syscall.SIGINT)
	<-c

	// Doesn't block if no connections, otherwise will wait until the timeout deadline or connections to finish,
	// whichever comes first.
	ctx, cancel := context.WithTimeout(context.Background(), 15) // shutdown gracefully
	defer cancel()

	err = httpServer.Shutdown(ctx)
	if err != nil {
		log.Error().Err(err).Msg("could not shutdown server in timeout specified")
		return
	}

	log.Info().Msg("http server exited gracefully")
}

// Used by the sdk to get environment variables that are required by all extensions.
type ExtensionSystemConfig struct {
	// Secret is the auth key passed by the main Gofer application to prevent other actors from communicating
	// with the extensions.
	Secret string `required:"true" json:"-"`

	// Unique identifier for the extension.
	ID string `required:"true"`

	// The log_level this extension should emit. Logs are ingested into the main Gofer application and combined
	// with the main application logs.
	LogLevel string `split_words:"true" default:"info"`

	// Contains the raw bytes for a TLS cert used by the extension to authenticate clients.
	UseTLS  bool   `split_words:"true" required:"true"`
	TLSCert string `split_words:"true" json:"-"`
	TLSKey  string `split_words:"true" json:"-"`

	BindAddress string `default:"0.0.0.0:8082"`
	GoferHost   string `split_words:"true" default:"localhost:8080"`
}

// GetExtensionSystemConfig returns environment variables that all extensions require. aka "System variables"
func GetExtensionSystemConfig() (ExtensionSystemConfig, error) {
	config := ExtensionSystemConfig{}
	err := envconfig.Process("GOFER_EXTENSION_SYSTEM", &config)
	if err != nil {
		return ExtensionSystemConfig{}, err
	}

	return config, nil
}

func (config *ExtensionSystemConfig) validate() error {
	if config.UseTLS {
		if len(config.TLSCert) == 0 {
			return fmt.Errorf("env var 'tls_cert' required but missing due to 'use_tls=true'")
		}
		if len(config.TLSKey) == 0 {
			return fmt.Errorf("env var 'tls_key' required but missing due to 'use_tls=true'")
		}
	}

	return nil
}

// Convenience function for grabbing the extension specific config value from the environment.
// Gofer passes in these values into the environment when the extension first starts.
func GetConfigFromEnv(key string) string {
	return os.Getenv(strings.ToUpper(key))
}

// setupLogging inits a global logging configuration that is used by all extensions.
// Ideally we'd want to offer the caller some way to log through the package,
// but since Go doesn't have good log interfaces we can just set this up by default
// and suggest they use this.
func setupLogging(extensionID, loglevel string) {
	zerolog.TimeFieldFormat = zerolog.TimeFormatUnix
	log.Logger = log.With().Caller().Logger()
	log.Logger = log.With().Str("extension", extensionID).Logger()
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

func (e *extensionWrapper) authMiddleware(next http.Handler) http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		// Skip if requested for health endpoint
		if r.URL.Path == "/api/health" {
			next.ServeHTTP(w, r)
			return
		}

		authHeader := r.Header.Get("Authorization")
		if authHeader == "" {
			http.Error(w, "Authorization header not found but required", http.StatusBadRequest)
			return
		}

		if !startsWithBearer(authHeader) {
			http.Error(w, "Authorization header malformed; should start with 'Bearer'", http.StatusBadRequest)
			return
		}

		token := authHeader[7:]
		if token != e.authKey {
			http.Error(w, "Unauthorized", http.StatusUnauthorized)
			return
		}

		next.ServeHTTP(w, r)
	})
}

// Helper function to check if the Authorization header starts with "Bearer "
func startsWithBearer(authHeader string) bool {
	return len(authHeader) > 6 && authHeader[:7] == "Bearer "
}

// The logging middleware has to be run before the final call to return the request.
// This is because we wrap the responseWriter to gain information from it after it
// has been written to.
// To speed this process up we call Serve as soon as possible and log afterwards.
func loggingMiddleware(next http.Handler) http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		start := time.Now()

		ww := middleware.NewWrapResponseWriter(w, r.ProtoMajor)
		next.ServeHTTP(ww, r)

		log.Debug().Str("method", r.Method).
			Stringer("url", r.URL).
			Int("status_code", ww.Status()).
			Int("response_size_bytes", ww.BytesWritten()).
			Dur("elapsed_ms", time.Since(start)).
			Msg("")
	})
}
