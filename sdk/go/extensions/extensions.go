package extensions

import (
	"context"
	"crypto/tls"
	"net/http"
	"os"
	"os/signal"
	"syscall"
	"time"

	"github.com/danielgtaylor/huma/v2"
	"github.com/danielgtaylor/huma/v2/adapters/humago"
	"github.com/go-chi/chi/v5/middleware"

	"github.com/kelseyhightower/envconfig"
	"github.com/rs/zerolog"
	"github.com/rs/zerolog/log"
)

type ExtensionError struct {
	Status  int    `json:"status" example:"401" doc:"The http error code for the error that occurred"`
	Message string `json:"message" example:"Could not get git repository" doc:"Description of error that occurred"`
}

func (e *ExtensionError) Error() string {
	return e.Message
}

func (e *ExtensionError) GetStatus() int {
	return e.Status
}

func NewExtensionError(status int, message string) *ExtensionError {
	return &ExtensionError{
		Status:  status,
		Message: message,
	}
}

type ExtensionInitRequest struct {
	Body struct {
		Config map[string]string `json:"config" example:"{\"interval\": \"5m\"}" doc:"The extension specific configuration for your pipeline"`
	}
}
type ExtensionInitResponse struct{}

type (
	ExtensionInfoRequest  struct{}
	ExtensionInfoResponse struct {
		Body struct {
			Name                string   `json:"name" example:"cron" doc:"The unique extension identifier"`
			Documentation       string   `json:"documentation" doc:"Markdown string documentation for extension"`
			RegisteredPipelines []string `json:"registered_pipelines" example:"[\"pipeline_one\", \"pipeline_three\"]"`
		}
	}
)

type ExtensionSubscribeRequest struct {
	NamespaceID            string            `json:"namespace_id" example:"default" doc:"The unique identifier for the namespace of the pipeline you wish to target"`
	PipelineID             string            `json:"pipeline_id" example:"some_pipeline" doc:"The unique identifier for the pipeline you wish to target"`
	PipelineSubscriptionID string            `json:"pipeline_subscription_id" example:"every_5_seconds" doc:"A unique name for the extension/pipeline subscription you wish to target"`
	Config                 map[string]string `json:"config" example:"{\"interval\":\"5m\"}" doc:"Extension specific parameters for this pipeline"`
}
type ExtensionSubscribeResponse struct{}

type ExtensionUnsubscribeRequest struct {
	NamespaceID            string `json:"namespace_id" example:"default" doc:"The unique identifier for the namespace of the pipeline you wish to target"`
	PipelineID             string `json:"pipeline_id" example:"some_pipeline" doc:"The unique identifier for the pipeline you wish to target"`
	PipelineSubscriptionID string `json:"pipeline_subscription_id" example:"every_5_seconds" doc:"The unique name for the extension/pipeline subscription you wish to target"`
}
type ExtensionUnsubscribeResponse struct{}

type (
	ExtensionShutdownRequest  struct{}
	ExtensionShutdownResponse struct{}
)

type ExtensionExternalEventRequest struct {
	Payload []byte `json:"payload" doc:"The bytes of the response body for the external request"`
}
type ExtensionExternalEventResponse struct{}

type (
	ExtensionRunInstallerRequest  struct{}
	ExtensionRunInstallerResponse struct{}
)

type (
	ExtensionRunPipelineConfiguratorRequest  struct{}
	ExtensionRunPipelineConfiguratorResponse struct{}
)

// ExtensionServiceInterface provides a light wrapper around the GRPC extension interface. This light wrapper
// provides the caller with a clear interface to implement and allows this package to bake in common
// functionality among all extensions.
type ExtensionServiceInterface interface {
	// Init tells the extension it should complete it's initialization phase and return when it is ready to serve requests.
	// This is useful because sometimes we'll want to start the extension, but not actually have it do anything
	// but serve only certain routes like the installation routes.
	Init(context.Context, *ExtensionInitRequest) (*ExtensionInitResponse, *ExtensionError)

	// Info returns information on the specific plugin
	Info(context.Context, *ExtensionInfoRequest) (*ExtensionInfoResponse, *ExtensionError)

	// Subscribe registers a pipeline with said extension to provide the extension's functionality.
	Subscribe(context.Context, *ExtensionSubscribeRequest) (*ExtensionSubscribeResponse, *ExtensionError)

	// Unsubscribe allows pipelines to remove their extension subscriptions.
	Unsubscribe(context.Context, *ExtensionUnsubscribeRequest) (*ExtensionUnsubscribeResponse, *ExtensionError)

	// Shutdown tells the extension to cleanup and gracefully shutdown. If a extension
	// does not shutdown in a time defined by the Gofer API the extension will
	// instead be Force shutdown(SIGKILL). This is to say that all extensions should
	// lean toward quick cleanups and shutdowns.
	Shutdown(context.Context, *ExtensionShutdownRequest) (*ExtensionShutdownResponse, *ExtensionError)

	// ExternalEvent are json blobs of Gofer's /events endpoint. Normally webhooks.
	ExternalEvent(context.Context, *ExtensionExternalEventRequest) (*ExtensionExternalEventResponse, *ExtensionError)

	// Run the installer that helps admin user install the extension.
	RunExtensionInstaller(context.Context, *ExtensionRunInstallerRequest) (*ExtensionRunInstallerResponse, *ExtensionError)

	// Run the installer that helps pipeline users with their pipeline extension
	// configuration.
	RunPipelineConfigurator(context.Context, *ExtensionRunPipelineConfiguratorRequest) (*ExtensionRunPipelineConfiguratorResponse, *ExtensionError)
}

type extension struct {
	isInitialized bool

	// Authentication key passed by the Gofer server for every extension.
	// Prevents out-of-band/external changes to extensions and provides
	// auth for extensions communicating back to Gofer.
	authKey string

	stop chan os.Signal
	impl ExtensionServiceInterface
}

func (e *extension) registerInit(apiDesc huma.API) {
	// Description //
	huma.Register(apiDesc, huma.Operation{
		OperationID:   "Init",
		Method:        http.MethodPost,
		Path:          "/api/init",
		DefaultStatus: 200,
		Summary:       "Initialize extension",
		Description: "Init tells the extension it should complete it's initialization phase and return when it is ready " +
			"to serve requests. This is useful because sometimes (namely when we are helping the user set up this extension) " +
			"we'll want to start the extension, but not actually have it do anything but server only certain routes like the " +
			"installation routes.",
		// Handler //
	}, func(ctx context.Context, request *ExtensionInitRequest) (*ExtensionInitResponse, error) {
		if e.isInitialized {
			return nil, huma.NewError(http.StatusConflict, "extension already initialized")
		}

		resp, err := e.impl.Init(ctx, request)
		if err != nil {
			return nil, err
		}

		if resp == nil {
			return nil, nil
		}

		e.isInitialized = true

		return resp, nil
	})
}

func (e *extension) registerInfo(apiDesc huma.API) {
	// Description //
	huma.Register(apiDesc, huma.Operation{
		OperationID:   "Info",
		Method:        http.MethodGet,
		Path:          "/api/info",
		DefaultStatus: 200,
		Summary:       "Returns general information about extension",
		Description:   "Return general information about the extension",
		// Handler //
	}, func(ctx context.Context, request *ExtensionInfoRequest) (*ExtensionInfoResponse, error) {
		resp, err := e.impl.Info(ctx, request)
		if err != nil {
			return nil, err
		}

		if resp == nil {
			return nil, nil
		}

		resp.Body.Name = os.Getenv("GOFER_EXTENSION_SYSTEM_NAME")

		return resp, nil
	})
}

func (e *extension) registerSubscribe(apiDesc huma.API) {
	// Description //
	huma.Register(apiDesc, huma.Operation{
		OperationID:   "Subscribe",
		Method:        http.MethodPost,
		Path:          "/api/subscribe",
		DefaultStatus: 201,
		Summary:       "Registers a pipeline with extension to provide the extension's functionality",
		Description:   "Registers a pipeline with extension to provide the extension's functionality",
		// Handler //
	}, func(ctx context.Context, request *ExtensionSubscribeRequest) (*ExtensionSubscribeResponse, error) {
		if !e.isInitialized {
			return nil, huma.NewError(http.StatusServiceUnavailable, "extension is not initialized yet")
		}

		resp, err := e.impl.Subscribe(ctx, request)
		if err != nil {
			return nil, err
		}

		if resp == nil {
			return nil, nil
		}

		return resp, nil
	})
}

func (e *extension) registerUnsubscribe(apiDesc huma.API) {
	// Description //
	huma.Register(apiDesc, huma.Operation{
		OperationID:   "Unsubscribe",
		Method:        http.MethodDelete,
		Path:          "/api/subscribe",
		DefaultStatus: 200,
		Summary:       "Remove a pipeline subscription from an extension",
		Description:   "Remove a pipeline subscription from an extension",
		// Handler //
	}, func(ctx context.Context, request *ExtensionUnsubscribeRequest) (*ExtensionUnsubscribeResponse, error) {
		if !e.isInitialized {
			return nil, huma.NewError(http.StatusServiceUnavailable, "extension is not initialized yet")
		}

		resp, err := e.impl.Unsubscribe(ctx, request)
		if err != nil {
			return nil, err
		}

		if resp == nil {
			return nil, nil
		}

		return resp, nil
	})
}

func (e *extension) registerShutdown(apiDesc huma.API) {
	// Description //
	huma.Register(apiDesc, huma.Operation{
		OperationID:   "Shutdown",
		Method:        http.MethodPost,
		Path:          "/api/shutdown",
		DefaultStatus: 200,
		Summary:       "Cleanup and gracefully shutdown extension",
		Description: "Shutdown tells the extension to cleanup and gracefully shutdown. If a extension does not shutdown " +
			"in a time defined by the Gofer API the extension will instead be forced(SIGKILL). This is to say that all extensions " +
			"should lean toward quick cleanups and shutdowns.",
		// Handler //
	}, func(ctx context.Context, request *ExtensionShutdownRequest) (*ExtensionShutdownResponse, error) {
		resp, err := e.impl.Shutdown(ctx, request)
		if err != nil {
			return nil, err
		}

		e.stop <- syscall.SIGTERM
		return resp, nil
	})
}

func (e *extension) registerExternalEvent(apiDesc huma.API) {
	// Description //
	huma.Register(apiDesc, huma.Operation{
		OperationID:   "ExternalEvent",
		Method:        http.MethodPost,
		Path:          "/api/external-events",
		DefaultStatus: 200,
		Summary:       "Insert event from Gofer's ExternalEvent endpoint",
		Description: "Extensions are allowed to receive payloads from external sources. These external sources are usually " +
			"providing said payloads via webhooks. The request initially gets sent to Gofer(via it's external event endpoint) " +
			"and then is relayed to the correct extensions to be processed and handled.",
		// Handler //
	}, func(ctx context.Context, request *ExtensionExternalEventRequest) (*ExtensionExternalEventResponse, error) {
		if !e.isInitialized {
			return nil, huma.NewError(http.StatusServiceUnavailable, "extension is not initialized yet")
		}

		resp, err := e.impl.ExternalEvent(ctx, request)
		if err != nil {
			return nil, err
		}

		if resp == nil {
			return nil, nil
		}

		return resp, nil
	})
}

func (e *extension) registerRunExtensionInstaller(apiDesc huma.API) {
	// Description //
	huma.Register(apiDesc, huma.Operation{
		OperationID:   "RunExtensionInstaller",
		Method:        http.MethodPost,
		Path:          "/api/run-extension-installer",
		DefaultStatus: 200,
		Summary:       "Run the installer that helps admin users install this extension",
		Description:   "Run the installer that helps admin users install this extension. Uses websockets.",
		// Handler //
	}, func(ctx context.Context, request *ExtensionRunInstallerRequest) (*ExtensionRunInstallerResponse, error) {
		resp, err := e.impl.RunExtensionInstaller(ctx, request)
		if err != nil {
			return nil, err
		}

		return resp, nil
	})
}

func (e *extension) registerRunPipelineConfigurator(apiDesc huma.API) {
	// Description //
	huma.Register(apiDesc, huma.Operation{
		OperationID:   "RunPipelineConfigurator",
		Method:        http.MethodPost,
		Path:          "/api/run-pipeline-configurator",
		DefaultStatus: 200,
		Summary:       "Run the installer that helps pipeline users with their pipeline extension",
		Description:   "Run the installer that helps pipeline users with their pipeline extension.",
		// Handler //
	}, func(ctx context.Context, request *ExtensionRunPipelineConfiguratorRequest) (*ExtensionRunPipelineConfiguratorResponse, error) {
		resp, err := e.impl.RunPipelineConfigurator(ctx, request)
		if err != nil {
			return nil, err
		}

		return resp, nil
	})
}

// NewExtension starts the provided extension service
func NewExtension(impl ExtensionServiceInterface) {
	config, err := GetExtensionSystemConfig()
	if err != nil {
		log.Fatal().Err(err).Msg("could not get environment variables for config")
	}

	setupLogging(config.Name, config.LogLevel)

	extensionServer := &extension{
		authKey: config.Secret,
		stop:    make(chan os.Signal, 1),
		impl:    impl,
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
func InitRouter(e *extension, name string) (router *http.ServeMux, apiDescription huma.API) {
	router = http.NewServeMux()

	humaConfig := huma.DefaultConfig(name, "")
	humaConfig.DocsPath = "/api/docs"
	humaConfig.OpenAPIPath = "/api/docs/openapi"

	apiDescription = humago.New(router, humaConfig)

	/* /api */

	e.registerInit(apiDescription)
	e.registerInfo(apiDescription)
	e.registerSubscribe(apiDescription)
	e.registerUnsubscribe(apiDescription)
	e.registerShutdown(apiDescription)
	e.registerExternalEvent(apiDescription)
	e.registerRunExtensionInstaller(apiDescription)
	e.registerRunPipelineConfigurator(apiDescription)

	return router, apiDescription
}

// run creates a grpc server with all the proper settings; TLS enabled
func (e *extension) run() {
	config, _ := GetExtensionSystemConfig()
	tlsConfig := getTLS()

	router, _ := InitRouter(e, config.Name)

	httpServer := http.Server{
		Addr:         config.Host,
		Handler:      loggingMiddleware(router),
		WriteTimeout: 15 * time.Second,
		ReadTimeout:  15 * time.Second,
		IdleTimeout:  15 * time.Second,
		TLSConfig:    tlsConfig,
	}

	// Run our server in a goroutine and listen for signals that indicate graceful shutdown
	go func() {
		if err := httpServer.ListenAndServeTLS("", ""); err != nil && err != http.ErrServerClosed {
			log.Fatal().Err(err).Msg("server exited abnormally")
		}
	}()
	log.Info().Str("url", config.Host).Msg("started extension http service")

	c := make(chan os.Signal, 1)
	signal.Notify(c, syscall.SIGTERM, syscall.SIGINT)
	<-c

	// Doesn't block if no connections, otherwise will wait until the timeout deadline or connections to finish,
	// whichever comes first.
	ctx, cancel := context.WithTimeout(context.Background(), 15) // shutdown gracefully
	defer cancel()

	err := httpServer.Shutdown(ctx)
	if err != nil {
		log.Error().Err(err).Msg("could not shutdown server in timeout specified")
		return
	}

	log.Info().Msg("http server exited gracefully")
}

// Used by the sdk to get environment variables that are required by all extensions.
type ExtensionSystemConfig struct {
	// Secret is the auth key passed by the main gofer application to prevent other
	// actors from attempting to communicate with the extensions.
	Secret string `required:"true" json:"-"`
	Name   string `required:"true"`

	// Possible values "debug", "info", "warn", "error", "fatal", "panic"
	LogLevel string `split_words:"true" default:"info"`

	// Contains the raw bytes for a TLS cert used by the extension to authenticate clients.
	TLSCert string `split_words:"true" required:"true" json:"-"`
	TLSKey  string `split_words:"true" required:"true" json:"-"`

	// Skip verification of TLS cert; useful for development.
	SkipTLSVerify bool   `split_words:"true" default:"false"`
	Host          string `default:"0.0.0.0:8082"`
	GoferHost     string `split_words:"true" default:"localhost:8080"`
}

// GetExtensionSystemConfig returns environment variables that all extensions require. aka "System variables"
func GetExtensionSystemConfig() (ExtensionSystemConfig, error) {
	config := ExtensionSystemConfig{}
	err := envconfig.Process("gofer_extension_system", &config)
	if err != nil {
		return ExtensionSystemConfig{}, err
	}

	return config, nil
}

// setupLogging inits a global logging configuration that is used by all extensions.
// Ideally we'd want to offer the caller some way to log through the package,
// but since Go doesn't have good log interfaces we can just set this up by default
// and suggest they use this.
func setupLogging(extensionName, loglevel string) {
	zerolog.TimeFieldFormat = zerolog.TimeFormatUnix
	log.Logger = log.With().Caller().Logger()
	log.Logger = log.With().Str("extension", extensionName).Logger()
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

// // Convenience function for sending a message to the client without excessive bulk.
// func SendConfiguratorMessageToClient(stream proto.ExtensionService_RunPipelineConfiguratorServer, msg string) error {
// 	err := stream.Send(&proto.ExtensionRunPipelineConfiguratorExtensionMessage{
// 		MessageType: &proto.ExtensionRunPipelineConfiguratorExtensionMessage_Msg{
// 			Msg: msg,
// 		},
// 	})
// 	if err != nil {
// 		return err
// 	}

// 	return nil
// }

// // Convenience function for sending a query to the client without excessive bulk.
// func SendConfiguratorQueryToClient(stream proto.ExtensionService_RunPipelineConfiguratorServer, query string) error {
// 	err := stream.Send(&proto.ExtensionRunPipelineConfiguratorExtensionMessage{
// 		MessageType: &proto.ExtensionRunPipelineConfiguratorExtensionMessage_Query{
// 			Query: query,
// 		},
// 	})
// 	if err != nil {
// 		return err
// 	}

// 	return nil
// }

// // Convenience function for sending a message to the client without excessive bulk.
// func SendConfiguratorParamSettingToClient(stream proto.ExtensionService_RunPipelineConfiguratorServer, param, value string) error {
// 	err := stream.Send(&proto.ExtensionRunPipelineConfiguratorExtensionMessage{
// 		MessageType: &proto.ExtensionRunPipelineConfiguratorExtensionMessage_ParamSetting_{
// 			ParamSetting: &proto.ExtensionRunPipelineConfiguratorExtensionMessage_ParamSetting{
// 				Param: param,
// 				Value: value,
// 			},
// 		},
// 	})
// 	if err != nil {
// 		return err
// 	}

// 	return nil
// }

// // Convenience function for sending a message to the client without excessive bulk.
// func SendInstallerMessageToClient(stream proto.ExtensionService_RunExtensionInstallerServer, msg string) error {
// 	err := stream.Send(&proto.ExtensionRunExtensionInstallerExtensionMessage{
// 		MessageType: &proto.ExtensionRunExtensionInstallerExtensionMessage_Msg{
// 			Msg: msg,
// 		},
// 	})
// 	if err != nil {
// 		return err
// 	}

// 	return nil
// }

// // Convenience function for sending a query to the client without excessive bulk.
// func SendInstallerQueryToClient(stream proto.ExtensionService_RunExtensionInstallerServer, query string) error {
// 	err := stream.Send(&proto.ExtensionRunExtensionInstallerExtensionMessage{
// 		MessageType: &proto.ExtensionRunExtensionInstallerExtensionMessage_Query{
// 			Query: query,
// 		},
// 	})
// 	if err != nil {
// 		return err
// 	}

// 	return nil
// }

// // Convenience function for sending a message to the client without excessive bulk.
// func SendInstallerConfigSettingToClient(stream proto.ExtensionService_RunExtensionInstallerServer, config, value string) error {
// 	err := stream.Send(&proto.ExtensionRunExtensionInstallerExtensionMessage{
// 		MessageType: &proto.ExtensionRunExtensionInstallerExtensionMessage_ConfigSetting_{
// 			ConfigSetting: &proto.ExtensionRunExtensionInstallerExtensionMessage_ConfigSetting{
// 				Config: config,
// 				Value:  value,
// 			},
// 		},
// 	})
// 	if err != nil {
// 		return err
// 	}

// 	return nil
// }

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
