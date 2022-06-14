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

	"github.com/clintjedwards/gofer/gofer_sdk/go/proto"
	grpc_middleware "github.com/grpc-ecosystem/go-grpc-middleware"
	grpc_auth "github.com/grpc-ecosystem/go-grpc-middleware/auth"
	"github.com/kelseyhightower/envconfig"
	"github.com/rs/zerolog"
	"github.com/rs/zerolog/log"
	"google.golang.org/grpc"
	"google.golang.org/grpc/codes"
	"google.golang.org/grpc/credentials"
	"google.golang.org/grpc/reflection"
	"google.golang.org/grpc/status"
)

// TriggerServiceInterface provides a light wrapper around the GRPC trigger interface. This light wrapper
// provides the caller with a clear interface to implement and allows this package to bake in common
// functionality among all triggers.
type TriggerServiceInterface interface {
	// Watch blocks until the trigger has a pipeline that should be run, then it returns. This is ideal for setting
	// the watch endpoint as an channel result.
	Watch(context.Context, *proto.TriggerWatchRequest) (*proto.TriggerWatchResponse, error)

	// Info returns information on the specific plugin
	Info(context.Context, *proto.TriggerInfoRequest) (*proto.TriggerInfoResponse, error)

	// Subscribe allows a trigger to keep track of all pipelines currently
	// dependant on that trigger so that we can trigger them at appropriate times.
	Subscribe(context.Context, *proto.TriggerSubscribeRequest) (*proto.TriggerSubscribeResponse, error)

	// Unsubscribe allows pipelines to remove their trigger subscriptions. This is
	// useful if the pipeline no longer needs to be notified about a specific
	// trigger automation.
	Unsubscribe(context.Context, *proto.TriggerUnsubscribeRequest) (*proto.TriggerUnsubscribeResponse, error)

	// Shutdown tells the trigger to cleanup and gracefully shutdown. If a trigger
	// does not shutdown in a time defined by the gofer API the trigger will
	// instead be Force shutdown(SIGKILL). This is to say that all triggers should
	// lean toward quick cleanups and shutdowns.
	Shutdown(context.Context, *proto.TriggerShutdownRequest) (*proto.TriggerShutdownResponse, error)

	// ExternalEvent are json blobs of gofer's /events endpoint. Normally
	// webhooks.
	ExternalEvent(context.Context, *proto.TriggerExternalEventRequest) (*proto.TriggerExternalEventResponse, error)
}

type trigger struct {
	// Authentication key passed by the Gofer server for every trigger.
	// Prevents out-of-band/external changes to triggers.
	authKey string

	stop chan os.Signal
	impl TriggerServiceInterface

	// We use "Unsafe" instead of "Unimplemented" due to unsafe forcing us to sacrifice forward compatibility in an
	// effort to be more correct in implementation.
	proto.UnsafeTriggerServiceServer
}

func (t *trigger) Watch(ctx context.Context, req *proto.TriggerWatchRequest) (*proto.TriggerWatchResponse, error) {
	resp, err := t.impl.Watch(ctx, req)
	if err != nil {
		return &proto.TriggerWatchResponse{}, err
	}

	if resp == nil {
		return &proto.TriggerWatchResponse{}, nil
	}

	return resp, nil
}

func (t *trigger) Info(ctx context.Context, req *proto.TriggerInfoRequest) (*proto.TriggerInfoResponse, error) {
	resp, err := t.impl.Info(ctx, req)
	if err != nil {
		return &proto.TriggerInfoResponse{}, err
	}

	if resp == nil {
		return &proto.TriggerInfoResponse{}, nil
	}

	resp.Name = os.Getenv("GOFER_TRIGGER_NAME")

	return resp, nil
}

func (t *trigger) Subscribe(ctx context.Context, req *proto.TriggerSubscribeRequest) (*proto.TriggerSubscribeResponse, error) {
	resp, err := t.impl.Subscribe(ctx, req)
	if err != nil {
		return &proto.TriggerSubscribeResponse{}, err
	}

	if resp == nil {
		return &proto.TriggerSubscribeResponse{}, nil
	}

	return resp, nil
}

func (t *trigger) Unsubscribe(ctx context.Context, req *proto.TriggerUnsubscribeRequest) (*proto.TriggerUnsubscribeResponse, error) {
	resp, err := t.impl.Unsubscribe(ctx, req)
	if err != nil {
		return &proto.TriggerUnsubscribeResponse{}, err
	}

	if resp == nil {
		return &proto.TriggerUnsubscribeResponse{}, nil
	}

	return resp, nil
}

func (t *trigger) Shutdown(ctx context.Context, req *proto.TriggerShutdownRequest) (*proto.TriggerShutdownResponse, error) {
	resp, err := t.impl.Shutdown(ctx, req)
	if err != nil {
		return nil, err
	}

	t.stop <- syscall.SIGTERM
	return resp, nil
}

func (t *trigger) ExternalEvent(ctx context.Context, req *proto.TriggerExternalEventRequest) (*proto.TriggerExternalEventResponse, error) {
	resp, err := t.impl.ExternalEvent(ctx, req)
	if err != nil {
		return &proto.TriggerExternalEventResponse{}, err
	}

	if resp == nil {
		return &proto.TriggerExternalEventResponse{}, nil
	}

	return resp, nil
}

func NewTrigger(service TriggerServiceInterface, installInstructions InstallInstructions) {
	if len(os.Args) != 2 {
		log.Fatal().Msg("Usage: ./trigger <server|installer>")
	}

	switch os.Args[1] {
	case "server":
		newTriggerServer(service)
	case "installer":
		instructions, err := installInstructions.JSON()
		if err != nil {
			log.Fatal().Msg("could not parse instructions to json")
		}
		fmt.Println(instructions)
	default:
		log.Fatal().Msg("Usage: ./trigger <server|installer>")
	}
}

// newTriggerServer starts the provided trigger service
func newTriggerServer(impl TriggerServiceInterface) {
	config, err := getTriggerConfig()
	if err != nil {
		log.Fatal().Err(err).Msg("could not get environment variables for config")
	}

	setupLogging(config.Name, config.LogLevel)

	triggerServer := &trigger{
		authKey: config.Key,
		stop:    make(chan os.Signal, 1),
		impl:    impl,
	}
	triggerServer.run()
}

// getTLS finds the certificates which are appropriate and
func getTLS() *tls.Config {
	config, _ := getTriggerConfig()

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
func (t *trigger) authFunc(ctx context.Context) (context.Context, error) {
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
func (t *trigger) run() {
	config, _ := getTriggerConfig()

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
	proto.RegisterTriggerServiceServer(server, t)

	listen, err := net.Listen("tcp", config.Host)
	if err != nil {
		log.Fatal().Err(err).Msg("could not init tcp listener")
	}

	log.Info().Str("url", config.Host).Msg("starting trigger grpc service")

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

// Used by the sdk to get environment variables that are required by all triggers.
type internalTriggerConfig struct {
	// Key is the auth key passed by the main gofer application to prevent other
	// actors from attempting to communicate with the triggers.
	Key  string `required:"true" json:"-"`
	Name string `required:"true"`
	// Possible values "debug", "info", "warn", "error", "fatal", "panic"
	LogLevel string `split_words:"true" default:"info"`
	// Contains the raw bytes for a TLS cert used by the trigger to authenticate clients.
	TLSCert string `split_words:"true" required:"true" json:"-"`
	TLSKey  string `split_words:"true" required:"true" json:"-"`
	Host    string `default:"0.0.0.0:8080"`
}

// getTriggerConfig returns environment variables that all triggers require.
func getTriggerConfig() (*internalTriggerConfig, error) {
	config := internalTriggerConfig{}
	err := envconfig.Process("gofer_trigger", &config)
	if err != nil {
		return nil, err
	}

	return &config, nil
}

// setupLogging inits a global logging configuration that is used by all triggers.
// Ideally we'd want to offer the caller some way to log through the package,
// but since Go doesn't have good log interfaces we can just set this up by default
// and suggest they use this.
func setupLogging(triggerName, loglevel string) {
	zerolog.TimeFieldFormat = zerolog.TimeFormatUnix
	log.Logger = log.With().Caller().Logger()
	log.Logger = log.With().Str("trigger", triggerName).Logger()
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

// GetConfig is a convenience function that returns trigger/notifier config values from the environment.
// It simply puts the needed config in the correct format to be retrieved from the environment
// so the caller doesn't have to.
func GetConfig(variable string) string {
	name := os.Getenv("GOFER_TRIGGER_NAME")
	return os.Getenv(fmt.Sprintf("GOFER_TRIGGER_%s_%s", strings.ToUpper(name), strings.ToUpper(variable)))
}

// InfoResponse is a convenience function for the Info interface function response.
func InfoResponse(documentationLink string) (*proto.TriggerInfoResponse, error) {
	return &proto.TriggerInfoResponse{
		Name:          os.Getenv("GOFER_TRIGGER_NAME"),
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
// At the end of the trigger setup, this response is used as part of the config
// to send to the trigger for installation.
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
