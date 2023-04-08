package sdk

import (
	"context"
	"crypto/tls"
	"encoding/json"
	"fmt"
	"net"
	"os"
	"os/signal"
	"strings"
	"syscall"
	"time"

	proto "github.com/clintjedwards/gofer/proto/go"

	grpc_middleware "github.com/grpc-ecosystem/go-grpc-middleware"
	grpc_auth "github.com/grpc-ecosystem/go-grpc-middleware/auth"
	"github.com/kelseyhightower/envconfig"
	"github.com/rs/zerolog"
	"github.com/rs/zerolog/log"
	"google.golang.org/grpc"
	"google.golang.org/grpc/codes"
	"google.golang.org/grpc/credentials"
	"google.golang.org/grpc/metadata"
	"google.golang.org/grpc/reflection"
	"google.golang.org/grpc/status"
)

// ExtensionServiceInterface provides a light wrapper around the GRPC extension interface. This light wrapper
// provides the caller with a clear interface to implement and allows this package to bake in common
// functionality among all extensions.
type ExtensionServiceInterface interface {
	// Info returns information on the specific plugin
	Info(context.Context, *proto.ExtensionInfoRequest) (*proto.ExtensionInfoResponse, error)

	// Subscribe registers a pipeline with said extension to provide the extension's functionality.
	Subscribe(context.Context, *proto.ExtensionSubscribeRequest) (*proto.ExtensionSubscribeResponse, error)

	// Unsubscribe allows pipelines to remove their extension subscriptions.
	Unsubscribe(context.Context, *proto.ExtensionUnsubscribeRequest) (*proto.ExtensionUnsubscribeResponse, error)

	// Shutdown tells the extension to cleanup and gracefully shutdown. If a extension
	// does not shutdown in a time defined by the Gofer API the extension will
	// instead be Force shutdown(SIGKILL). This is to say that all extensions should
	// lean toward quick cleanups and shutdowns.
	Shutdown(context.Context, *proto.ExtensionShutdownRequest) (*proto.ExtensionShutdownResponse, error)

	// ExternalEvent are json blobs of Gofer's /events endpoint. Normally webhooks.
	ExternalEvent(context.Context, *proto.ExtensionExternalEventRequest) (*proto.ExtensionExternalEventResponse, error)
}

type extension struct {
	// Authentication key passed by the Gofer server for every extension.
	// Prevents out-of-band/external changes to extensions.
	authKey string

	stop chan os.Signal
	impl ExtensionServiceInterface

	// We use "Unsafe" instead of "Unimplemented" due to unsafe forcing us to sacrifice forward compatibility in an
	// effort to be more correct in implementation.
	proto.UnsafeExtensionServiceServer
}

func (t *extension) Info(ctx context.Context, req *proto.ExtensionInfoRequest) (*proto.ExtensionInfoResponse, error) {
	resp, err := t.impl.Info(ctx, req)
	if err != nil {
		return &proto.ExtensionInfoResponse{}, err
	}

	if resp == nil {
		return &proto.ExtensionInfoResponse{}, nil
	}

	resp.Name = os.Getenv("GOFER_EXTENSION_SYSTEM_NAME")

	return resp, nil
}

func (t *extension) Subscribe(ctx context.Context, req *proto.ExtensionSubscribeRequest) (*proto.ExtensionSubscribeResponse, error) {
	resp, err := t.impl.Subscribe(ctx, req)
	if err != nil {
		return &proto.ExtensionSubscribeResponse{}, err
	}

	if resp == nil {
		return &proto.ExtensionSubscribeResponse{}, nil
	}

	return resp, nil
}

func (t *extension) Unsubscribe(ctx context.Context, req *proto.ExtensionUnsubscribeRequest) (*proto.ExtensionUnsubscribeResponse, error) {
	resp, err := t.impl.Unsubscribe(ctx, req)
	if err != nil {
		return &proto.ExtensionUnsubscribeResponse{}, err
	}

	if resp == nil {
		return &proto.ExtensionUnsubscribeResponse{}, nil
	}

	return resp, nil
}

func (t *extension) Shutdown(ctx context.Context, req *proto.ExtensionShutdownRequest) (*proto.ExtensionShutdownResponse, error) {
	resp, err := t.impl.Shutdown(ctx, req)
	if err != nil {
		return nil, err
	}

	t.stop <- syscall.SIGTERM
	return resp, nil
}

func (t *extension) ExternalEvent(ctx context.Context, req *proto.ExtensionExternalEventRequest) (*proto.ExtensionExternalEventResponse, error) {
	resp, err := t.impl.ExternalEvent(ctx, req)
	if err != nil {
		return &proto.ExtensionExternalEventResponse{}, err
	}

	if resp == nil {
		return &proto.ExtensionExternalEventResponse{}, nil
	}

	return resp, nil
}

func NewExtension(service ExtensionServiceInterface, installInstructions InstallInstructions) {
	if len(os.Args) != 2 {
		log.Fatal().Msg("Usage: ./extension <server|installer>")
	}

	switch os.Args[1] {
	case "server":
		newExtensionServer(service)
	case "installer":
		instructions, err := installInstructions.JSON()
		if err != nil {
			log.Fatal().Msg("could not parse instructions to json")
		}
		fmt.Println(instructions)
	default:
		log.Fatal().Msg("Usage: ./extension <server|installer>")
	}
}

// Connect to Gofer's API
func Connect() (proto.GoferClient, context.Context, error) {
	goferHost := os.Getenv("GOFER_EXTENSION_SYSTEM_GOFER_HOST")
	skipTLSVerify := os.Getenv("GOFER_EXTENSION_SYSTEM_SKIP_TLS_VERIFY")

	host, port, _ := strings.Cut(goferHost, ":")

	// If we are not given a port we assume that port is 443
	if port == "" {
		port = "443"
	}

	var opt []grpc.DialOption
	var tlsConf *tls.Config

	if skipTLSVerify == "true" {
		tlsConf = &tls.Config{
			InsecureSkipVerify: true,
		}
	}

	opt = append(opt, grpc.WithTransportCredentials(credentials.NewTLS(tlsConf)))
	conn, err := grpc.Dial(fmt.Sprintf("%s:%s", host, port), opt...)
	if err != nil {
		return nil, nil, fmt.Errorf("could not connect to server: %w", err)
	}

	client := proto.NewGoferClient(conn)

	key := os.Getenv("GOFER_EXTENSION_SYSTEM_KEY")

	md := metadata.Pairs("Authorization", "Bearer "+key)
	ctx := metadata.NewOutgoingContext(context.Background(), md)

	return client, ctx, nil
}

// newExtensionServer starts the provided extension service
func newExtensionServer(impl ExtensionServiceInterface) {
	config, err := GetExtensionSystemConfig()
	if err != nil {
		log.Fatal().Err(err).Msg("could not get environment variables for config")
	}

	setupLogging(config.Name, config.LogLevel)

	extensionServer := &extension{
		authKey: config.Key,
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

// authFunc is used to compare the key received from GRPC metadata with the key that
func (t *extension) authFunc(ctx context.Context) (context.Context, error) {
	token, err := grpc_auth.AuthFromMD(ctx, "bearer")
	if err != nil {
		return nil, err
	}

	if token != t.authKey {
		return nil, status.Errorf(codes.Unauthenticated, "invalid auth token")
	}

	return ctx, nil
}

// run creates a grpc server with all the proper settings; TLS enabled
func (t *extension) run() {
	config, _ := GetExtensionSystemConfig()

	server := grpc.NewServer(
		grpc.Creds(credentials.NewTLS(getTLS())),
		grpc.StreamInterceptor(
			grpc_middleware.ChainStreamServer(
				grpc_auth.StreamServerInterceptor(t.authFunc),
			),
		),
		grpc.UnaryInterceptor(
			grpc_middleware.ChainUnaryServer(
				grpc_auth.UnaryServerInterceptor(t.authFunc),
			),
		),
	)

	reflection.Register(server)
	proto.RegisterExtensionServiceServer(server, t)

	listen, err := net.Listen("tcp", config.Host)
	if err != nil {
		log.Fatal().Err(err).Msg("could not init tcp listener")
	}

	log.Info().Str("url", config.Host).Msg("starting extension grpc service")

	go func() {
		if err := server.Serve(listen); err != nil {
			log.Error().Err(err).Msg("server encountered an error")
		}
	}()

	signal.Notify(t.stop, syscall.SIGTERM, syscall.SIGINT)
	<-t.stop

	// shutdown gracefully with a timeout
	stopped := make(chan struct{})
	go func() {
		server.GracefulStop()
		close(stopped)
	}()

	timer := time.NewTicker(15 * time.Second)
	select {
	case <-timer.C:
		server.Stop()
	case <-stopped:
		timer.Stop()
	}
}

// Used by the sdk to get environment variables that are required by all extensions.
type ExtensionSystemConfig struct {
	// Key is the auth key passed by the main gofer application to prevent other
	// actors from attempting to communicate with the extensions.
	Key  string `required:"true" json:"-"`
	Name string `required:"true"`

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

// GetConfig is a convenience function that returns extension/commonTask config values from the environment.
// It simply puts the needed config in the correct format to be retrieved from the environment
// so the caller doesn't have to.
func GetConfig(variableName string) string {
	return os.Getenv(fmt.Sprintf("GOFER_EXTENSION_CONFIG_%s", strings.ToUpper(variableName)))
}

// GetParameters is a convenience function that returns extension/commonTask config values from the environment.
// It simply puts the needed config in the correct format to be retrieved from the environment
// so the caller doesn't have to.
func GetParameter(variableName string) string {
	return os.Getenv(fmt.Sprintf("GOFER_EXTENSION_PARAM_%s", strings.ToUpper(variableName)))
}

// InfoResponse is a convenience function for the Info interface function response.
func InfoResponse(documentationLink string) (*proto.ExtensionInfoResponse, error) {
	return &proto.ExtensionInfoResponse{
		Name:          os.Getenv("GOFER_EXTENSION_SYSTEM_NAME"),
		Documentation: documentationLink,
	}, nil
}

type isInstallInstruction interface {
	isInstallInstruction()
}

type InstallInstructionQueryWrapper struct {
	Query InstallInstructionQuery `json:"query"`
}

func (InstallInstructionQueryWrapper) isInstallInstruction() {}

type InstallInstructionQuery struct {
	Text      string `json:"text"`
	ConfigKey string `json:"config_key"`
}

type InstallInstructionMessageWrapper struct {
	Message InstallInstructionMessage `json:"message"`
}

func (InstallInstructionMessageWrapper) isInstallInstruction() {}

type InstallInstructionMessage struct {
	Text string `json:"text"`
}

type InstallInstructions struct {
	Instructions []isInstallInstruction `json:"instructions"`
}

func (i *InstallInstructions) UnmarshalJSON(b []byte) error {
	data := make(map[string]json.RawMessage)
	err := json.Unmarshal(b, &data)
	if err != nil {
		return err
	}

	// Peel back the "instructions"
	instructionJSON := data["instructions"]
	instructions := []json.RawMessage{}

	err = json.Unmarshal(instructionJSON, &instructions)
	if err != nil {
		return err
	}

	for _, value := range instructions {
		instruction := map[string]json.RawMessage{}

		err = json.Unmarshal(value, &instruction)
		if err != nil {
			return err
		}

		messageInstructionJSON, exists := instruction["message"]
		if exists {
			messageInstruction := InstallInstructionMessage{}
			err = json.Unmarshal(messageInstructionJSON, &messageInstruction)
			if err != nil {
				return err
			}

			i.Instructions = append(i.Instructions, InstallInstructionMessageWrapper{
				Message: messageInstruction,
			})

			continue
		}

		queryInstructionJSON, exists := instruction["query"]
		if exists {
			queryInstruction := InstallInstructionQuery{}
			err = json.Unmarshal(queryInstructionJSON, &queryInstruction)
			if err != nil {
				return err
			}

			i.Instructions = append(i.Instructions, InstallInstructionQueryWrapper{
				Query: queryInstruction,
			})

			continue
		}
	}

	return nil
}

func NewInstructionsBuilder() InstallInstructions {
	return InstallInstructions{
		Instructions: []isInstallInstruction{},
	}
}

// AddMessage adds a new generic text string for the installation instructions.
// This string is simply printed back to the reader.
func (i InstallInstructions) AddMessage(text string) InstallInstructions {
	i.Instructions = append(i.Instructions,
		InstallInstructionMessageWrapper{Message: InstallInstructionMessage{Text: text}})

	return i
}

// AddQuery adds a new text query for the installation instructions.
// This string is printed back to the client and then the CLI waits for a response.
// At the end of the extension setup, this response is used as part of the config
// to send to the extension for installation.
func (i InstallInstructions) AddQuery(text, configKey string) InstallInstructions {
	i.Instructions = append(i.Instructions,
		InstallInstructionQueryWrapper{Query: InstallInstructionQuery{Text: text, ConfigKey: configKey}})

	return i
}

// Take our current instructions and return a json string
func (i InstallInstructions) JSON() (string, error) {
	output, err := json.Marshal(i)
	if err != nil {
		return "", err
	}

	return string(output), nil
}
