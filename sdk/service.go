package sdk

import (
	"context"
	"crypto/tls"
	"net"
	"os"
	"os/signal"
	"syscall"
	"time"

	sdkProto "github.com/clintjedwards/gofer/sdk/proto"
	grpc_middleware "github.com/grpc-ecosystem/go-grpc-middleware"
	grpc_auth "github.com/grpc-ecosystem/go-grpc-middleware/auth"
	"github.com/rs/zerolog/log"
	"google.golang.org/grpc"
	"google.golang.org/grpc/codes"
	"google.golang.org/grpc/credentials"
	"google.golang.org/grpc/reflection"
	"google.golang.org/grpc/status"
)

// TriggerServerInterface provides a light wrapper around the GRPC trigger interface. This light wrapper
// provides the caller with a clear interface to implement and allows this package to bake in common
// functionality among all triggers.
type TriggerServerInterface interface {
	// Check blocks until the trigger has a pipeline that should be run, then it returns. This is ideal for setting
	// the check endpoint as an channel result.
	Check(context.Context, *sdkProto.CheckRequest) (*sdkProto.CheckResponse, error)

	// Info returns information on the specific plugin
	Info(context.Context, *sdkProto.InfoRequest) (*sdkProto.InfoResponse, error)

	// Subscribe allows a trigger to keep track of all pipelines currently
	// dependant on that trigger so that we can trigger them at appropriate times.
	Subscribe(context.Context, *sdkProto.SubscribeRequest) (*sdkProto.SubscribeResponse, error)

	// Unsubscribe allows pipelines to remove their trigger subscriptions. This is
	// useful if the pipeline no longer needs to be notified about a specific
	// trigger automation.
	Unsubscribe(context.Context, *sdkProto.UnsubscribeRequest) (*sdkProto.UnsubscribeResponse, error)

	// Shutdown tells the trigger to cleanup and gracefully shutdown. If a trigger
	// does not shutdown in a time defined by the gofer API the trigger will
	// instead be Force shutdown(SIGKILL). This is to say that all triggers should
	// lean toward quick cleanups and shutdowns.
	Shutdown(context.Context, *sdkProto.ShutdownRequest) (*sdkProto.ShutdownResponse, error)

	// ExternalEvent are json blobs of gofer's /events endpoint. Normally
	// webhooks.
	ExternalEvent(context.Context, *sdkProto.ExternalEventRequest) (*sdkProto.ExternalEventResponse, error)
}

type triggerServer struct {
	authKey string // Authentication key passed by the Gofer server for every trigger. Prevents out-of-band changes to triggers.

	stop chan os.Signal
	impl TriggerServerInterface

	// We use "Unsafe" instead of "Unimplemented" due to unsafe forcing us to sacrafice forward compatibility in an
	// effort to be more correct in implementation.
	sdkProto.UnsafeTriggerServer
}

func (t *triggerServer) Check(ctx context.Context, req *sdkProto.CheckRequest) (*sdkProto.CheckResponse, error) {
	resp, err := t.impl.Check(ctx, req)
	if err != nil {
		return &sdkProto.CheckResponse{}, err
	}

	if resp == nil {
		return &sdkProto.CheckResponse{}, nil
	}

	return resp, nil
}

func (t *triggerServer) Info(ctx context.Context, req *sdkProto.InfoRequest) (*sdkProto.InfoResponse, error) {
	resp, err := t.impl.Info(ctx, req)
	if err != nil {
		return &sdkProto.InfoResponse{}, err
	}

	if resp == nil {
		return &sdkProto.InfoResponse{}, nil
	}

	resp.Kind = os.Getenv("GOFER_TRIGGER_KIND")

	return resp, nil
}

func (t *triggerServer) Subscribe(ctx context.Context, req *sdkProto.SubscribeRequest) (*sdkProto.SubscribeResponse, error) {
	resp, err := t.impl.Subscribe(ctx, req)
	if err != nil {
		return &sdkProto.SubscribeResponse{}, err
	}

	if resp == nil {
		return &sdkProto.SubscribeResponse{}, nil
	}

	return resp, nil
}

func (t *triggerServer) Unsubscribe(ctx context.Context, req *sdkProto.UnsubscribeRequest) (*sdkProto.UnsubscribeResponse, error) {
	resp, err := t.impl.Unsubscribe(ctx, req)
	if err != nil {
		return &sdkProto.UnsubscribeResponse{}, err
	}

	if resp == nil {
		return &sdkProto.UnsubscribeResponse{}, nil
	}

	return resp, nil
}

func (t *triggerServer) Shutdown(ctx context.Context, req *sdkProto.ShutdownRequest) (*sdkProto.ShutdownResponse, error) {
	resp, err := t.impl.Shutdown(ctx, req)
	if err != nil {
		return nil, err
	}

	t.stop <- syscall.SIGTERM
	return resp, nil
}

func (t *triggerServer) ExternalEvent(ctx context.Context, req *sdkProto.ExternalEventRequest) (*sdkProto.ExternalEventResponse, error) {
	resp, err := t.impl.ExternalEvent(ctx, req)
	if err != nil {
		return &sdkProto.ExternalEventResponse{}, err
	}

	if resp == nil {
		return &sdkProto.ExternalEventResponse{}, nil
	}

	return resp, nil
}

func newTriggerServer(t TriggerServerInterface) {
	config, err := initConfig()
	if err != nil {
		log.Fatal().Err(err).Msg("could not get environment variables for config")
	}

	setupLogging(config.Kind, config.LogLevel)

	triggerServer := &triggerServer{
		authKey: config.Key,
		stop:    make(chan os.Signal, 1),
		impl:    t,
	}
	triggerServer.run()
}

// getTLS finds the certificates which are appropriate and
func getTLS() *tls.Config {
	config, _ := initConfig()

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
func (t *triggerServer) authFunc(ctx context.Context) (context.Context, error) {
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
func (t *triggerServer) run() {
	config, _ := initConfig()

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
	sdkProto.RegisterTriggerServer(server, t)

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
