// Package api controls the bulk of the Gofer API logic.
package api

import (
	"context"
	"crypto/tls"
	"errors"
	"fmt"
	"net/http"
	"os"
	"os/signal"
	"runtime/debug"
	"strings"
	"sync/atomic"
	"syscall"
	"time"

	"github.com/clintjedwards/gofer/internal/config"
	"github.com/clintjedwards/gofer/internal/eventbus"
	"github.com/clintjedwards/gofer/internal/objectStore"
	"github.com/clintjedwards/gofer/internal/scheduler"
	"github.com/clintjedwards/gofer/internal/secretStore"
	"github.com/clintjedwards/gofer/internal/storage"
	"github.com/clintjedwards/gofer/internal/syncmap"
	"github.com/clintjedwards/gofer/models"
	proto "github.com/clintjedwards/gofer/proto/go"
	"github.com/gorilla/handlers"
	"github.com/gorilla/mux"
	grpc_middleware "github.com/grpc-ecosystem/go-grpc-middleware"
	grpc_auth "github.com/grpc-ecosystem/go-grpc-middleware/auth"
	grpc_recovery "github.com/grpc-ecosystem/go-grpc-middleware/recovery"
	grpc_retry "github.com/grpc-ecosystem/go-grpc-middleware/retry"
	"github.com/improbable-eng/grpc-web/go/grpcweb"
	"github.com/rs/zerolog/log"
	"google.golang.org/grpc"
	"google.golang.org/grpc/codes"
	"google.golang.org/grpc/credentials"
	"google.golang.org/grpc/reflection"
	"google.golang.org/grpc/status"
)

func ptr[T any](v T) *T {
	return &v
}

var (
	// ErrPipelineNotActive is returned when a request is made against a pipeline that is not in the active state.
	ErrPipelineNotActive = errors.New("api: pipeline is not in state 'active'")

	// ErrPipelineActive is returned when a request is made against a pipeline in the active state.
	ErrPipelineActive = errors.New("api: pipeline is in state 'active'")

	// ErrPipelineRunsInProgress is returned when a request is made against a pipeline with currently in progress runs.
	ErrPipelineRunsInProgress = errors.New("api: pipeline has runs which are still in progress")

	// ErrPipelineConfigNotValid is returned when a pipeline configuration contains is not valid for the trigger requested.
	ErrPipelineConfigNotValid = errors.New("api: pipeline configuration is invalid")

	// ErrTriggerNotFound is returned when a pipeline configuration contains a trigger that was not registered with the API.
	ErrTriggerNotFound = errors.New("api: trigger is not found")
)

type CancelContext struct {
	ctx    context.Context
	cancel context.CancelFunc
}

// API represents the main Gofer service API. It is run using a GRPC/HTTP combined server.
// This main API handles 99% of interactions with the gofer service itself and is only missing the hooks for the
// gofer events service.
type API struct {
	// Parent context for management goroutines. Used to easily stop goroutines on shutdown.
	context *CancelContext

	// Config represents the relative configuration for the Gofer API. This is a combination of envvars and config values
	// gleaned at startup time.
	config *config.API

	// Storage represents the main backend storage implementation. Gofer stores most of its critical state information
	// using this storage mechanism.
	db storage.DB

	// Scheduler is the mechanism in which Gofer uses to run its individual containers. It leverages that backend
	// scheduler to do most of the work on running the user's task runs(docker containers).
	scheduler scheduler.Engine

	// ObjectStore is the mechanism in which Gofer stores pipeline and run level objects. The implementation here
	// is meant to act as a basic object store.
	objectStore objectStore.Engine

	// SecretStore is the mechanism in which Gofer pipeline secrets. This is the way in which users can fill pipeline
	// files with secrets.
	secretStore secretStore.Engine

	// TODO(clintjedwards): replace this syncmap with an actually good version once generics catches up.
	// Triggers is an in-memory map of currently registered triggers. These triggers are registered on startup and
	// launched as long running containers via the scheduler. Gofer refers to this cache as a way to communicate
	// quickly with the containers and their potentially changing endpoints.
	triggers syncmap.Syncmap[string, *models.Trigger]

	// commonTasks is an in-memory map of the currently registered commonTasks. These commonTasks are registered on startup
	// and launched as needed at a user's request. Gofer refers to this cache as a way to quickly look
	// up which container is needed to be launched.
	commonTasks syncmap.Syncmap[string, *models.CommonTaskRegistration]

	// ignorePipelineRunEvents controls if pipelines can trigger runs globally. If this is set to false the entire Gofer
	// service will not schedule new runs.
	ignorePipelineRunEvents *atomic.Bool

	// events acts as an event bus for the Gofer application. It is used throughout the whole application to give
	// different parts of the application the ability to listen for and respond to events that might happen in other
	// parts.
	events *eventbus.EventBus

	// We opt out of forward compatibility with this embedded interface. This is required by GRPC.
	//
	// We don't embed the "proto.UnimplementedGoferServer" as there should never(I assume this will come back to bite me)
	// be an instance where we add proto methods without also updating the server to support those methods.
	// There is the added benefit that without it embedded we get compile time errors when a function isn't correctly
	// implemented. Saving us from weird "Unimplemented" RPC bugs.
	proto.UnsafeGoferServer
}

// NewAPI creates a new instance of the main Gofer API service.
func NewAPI(config *config.API, storage storage.DB, scheduler scheduler.Engine, objectStore objectStore.Engine,
	secretStore secretStore.Engine,
) (*API, error) {
	eventbus, err := eventbus.New(storage, config.EventLogRetention, config.PruneEventsInterval)
	if err != nil {
		return nil, fmt.Errorf("could not init event bus: %w", err)
	}

	ctx, cancel := context.WithCancel(context.Background())

	var ignorePipelineRunEvents atomic.Bool
	ignorePipelineRunEvents.Store(config.IgnorePipelineRunEvents)

	newAPI := &API{
		context: &CancelContext{
			ctx:    ctx,
			cancel: cancel,
		},
		config:                  config,
		db:                      storage,
		events:                  eventbus,
		scheduler:               scheduler,
		objectStore:             objectStore,
		secretStore:             secretStore,
		ignorePipelineRunEvents: &ignorePipelineRunEvents,
		triggers:                syncmap.New[string, *models.Trigger](),
		commonTasks:             syncmap.New[string, *models.CommonTaskRegistration](),
	}

	err = newAPI.createDefaultNamespace()
	if err != nil {
		return nil, fmt.Errorf("could not create default namespace: %w", err)
	}

	err = newAPI.installBaseTriggers()
	if err != nil {
		return nil, fmt.Errorf("could not install base triggers: %w", err)
	}

	err = newAPI.startTriggers()
	if err != nil {
		return nil, fmt.Errorf("could not start triggers: %w", err)
	}

	err = newAPI.restoreTriggerSubscriptions()
	if err != nil {
		newAPI.cleanup()
		return nil, fmt.Errorf("could not restore trigger subscriptions: %w", err)
	}

	// findOrphans is a repair method that picks up where the gofer service left off if it was shutdown while
	// a run was currently in progress.
	// go newAPI.findOrphans()

	// These two functions are responsible for gofer's trigger event loop system. The first launches goroutines that
	// consumes events from triggers and the latter processes them into pipeline runs.
	newAPI.watchForTriggerEvents(newAPI.context.ctx)
	go func() {
		err := newAPI.processTriggerEvents()
		if err != nil {
			panic(err)
		}
	}()

	return newAPI, nil
}

// cleanup gracefully cleans up all goroutines to ensure a clean shutdown.
func (api *API) cleanup() {
	api.ignorePipelineRunEvents.Store(true)

	// Send graceful stop to all triggers
	api.stopTriggers()

	// Stop all goroutines which should stop the event processing pipeline and the trigger monitoring.
	api.context.cancel()
}

// StartAPIService starts the Gofer API service and blocks until a SIGINT or SIGTERM is received.
func (api *API) StartAPIService() {
	grpcServer, err := api.createGRPCServer()
	if err != nil {
		log.Fatal().Err(err).Msg("could not create GRPC service")
	}

	tlsConfig, err := api.generateTLSConfig(api.config.Server.TLSCertPath, api.config.Server.TLSKeyPath)
	if err != nil {
		log.Fatal().Err(err).Msg("could not get proper TLS config")
	}

	httpServer := wrapGRPCServer(api.config, grpcServer)
	httpServer.TLSConfig = tlsConfig

	// Run our server in a goroutine and listen for signals that indicate graceful shutdown
	go func() {
		if err := httpServer.ListenAndServeTLS("", ""); err != nil && err != http.ErrServerClosed {
			log.Fatal().Err(err).Msg("server exited abnormally")
		}
	}()
	log.Info().Str("url", api.config.Host).Msg("started gofer grpc/http service")

	c := make(chan os.Signal, 1)
	signal.Notify(c, syscall.SIGTERM, syscall.SIGINT)
	<-c

	// On ctrl-c we need to clean up not only the connections from the GRPC server, but make sure all the currently
	// running jobs are logged and exited properly.
	api.cleanup()

	// Doesn't block if no connections, otherwise will wait until the timeout deadline or connections to finish,
	// whichever comes first.
	ctx, cancel := context.WithTimeout(context.Background(), api.config.Server.ShutdownTimeout) // shutdown gracefully
	defer cancel()

	err = httpServer.Shutdown(ctx)
	if err != nil {
		log.Error().Err(err).Msg("could not shutdown server in timeout specified")
		return
	}

	log.Info().Msg("grpc server exited gracefully")
}

// wrapGRPCServer returns a combined grpc/http (grpc-web compatible) service with all proper settings;
// Rather than going through the trouble of setting up a separate proxy and extra for the service in order to server http/grpc/grpc-web
// this keeps things simple by enabling the operator to deploy a single binary and serve them all from one endpoint.
// This reduces operational burden, configuration headache and overall just makes for a better time for both client and operator.
func wrapGRPCServer(config *config.API, grpcServer *grpc.Server) *http.Server {
	wrappedGrpc := grpcweb.WrapServer(grpcServer)

	router := mux.NewRouter()

	combinedHandler := http.HandlerFunc(func(resp http.ResponseWriter, req *http.Request) {
		if strings.Contains(req.Header.Get("Content-Type"), "application/grpc") || wrappedGrpc.IsGrpcWebRequest(req) {
			wrappedGrpc.ServeHTTP(resp, req)
			return
		}
		router.ServeHTTP(resp, req)
	})

	var modifiedHandler http.Handler
	if config.Server.DevMode {
		modifiedHandler = handlers.LoggingHandler(os.Stdout, combinedHandler)
	} else {
		modifiedHandler = combinedHandler
	}

	httpServer := http.Server{
		Addr:    config.Host,
		Handler: modifiedHandler,
		// Timeouts set here unfortunately also apply to the backing GRPC server. Because GRPC might have long running calls
		// we have to set these to 0 or a very high number. This creates an issue where running the frontend in this configuration
		// could possibly open us up to DOS attacks where the client holds the request open for long periods of time. To mitigate
		// this we both implement timeouts for routes on both the GRPC side and the pure HTTP side.
		WriteTimeout: 0,
		ReadTimeout:  0,
	}

	return &httpServer
}

func (api *API) installBaseTriggers() error {
	if !api.config.Triggers.InstallBaseTriggers {
		return nil
	}

	registeredTriggers, err := api.db.ListTriggerRegistrations(0, 0)
	if err != nil {
		return err
	}

	cronInstalled := false
	intervalInstalled := false

	for _, trigger := range registeredTriggers {
		if strings.EqualFold(trigger.Name, "cron") {
			cronInstalled = true
		}

		if strings.EqualFold(trigger.Name, "interval") {
			intervalInstalled = true
		}
	}

	if !cronInstalled {
		registration := models.TriggerRegistration{}
		registration.FromInstallTriggerRequest(&proto.InstallTriggerRequest{
			Name:  "cron",
			Image: "ghcr.io/clintjedwards/gofer-containers/triggers/cron:latest",
		})

		err := api.db.InsertTriggerRegistration(&registration)
		if err != nil {
			if !errors.Is(err, storage.ErrEntityExists) {
				return err
			}
		}

		log.Info().Str("name", registration.Name).Str("image", registration.Image).
			Msg("registered base trigger automatically due to 'install_base_triggers' config")
	}

	if !intervalInstalled {
		registration := models.TriggerRegistration{}
		registration.FromInstallTriggerRequest(&proto.InstallTriggerRequest{
			Name:  "interval",
			Image: "ghcr.io/clintjedwards/gofer-containers/triggers/interval:latest",
		})

		err := api.db.InsertTriggerRegistration(&registration)
		if err != nil {
			if !errors.Is(err, storage.ErrEntityExists) {
				return err
			}
		}

		log.Info().Str("name", registration.Name).Str("image", registration.Image).
			Msg("registered base trigger automatically due to 'install_base_triggers' config")
	}

	return nil
}

// Gofer starts with a default namespace that all users have access to.
func (api *API) createDefaultNamespace() error {
	namespace := models.NewNamespace(namespaceDefaultID, namespaceDefaultName, "default namespace")
	err := api.db.InsertNamespace(namespace)
	if err != nil {
		if errors.Is(err, storage.ErrEntityExists) {
			return nil
		}

		return err
	}

	api.events.Publish(models.EventCreatedNamespace{
		NamespaceID: namespace.ID,
	})

	return nil
}

// findOrphans allows the gofer service to be shutdown and still pick back up where it left off on next startup.
// It does this by simply re-attaching the state monitoring go routines for a run and its child task runs.
// While simple on its face this is actually quite non-trivial as it requires delicate figuring out where the run is
// currently in its lifecycle and accounting for any state it could possibly be in.
//
// Gofer identifies runs that haven't fully completed by keeping an in-progress run "cache" and identifying which ones it
// did not get an opportunity to finish tracking.
//
// It then asks the scheduler for the last status of the container and appropriately either:
//   - If the run is unfinished: Attach the goroutine responsible for monitoring said run.
//   - If the container/task run is still running: Attach state watcher goroutine, truncate logs, attach new log watcher.
//   - If the container is in a finished state: Remove from run cache -> update container state -> clear out logs
//     -> update logs with new logs.
//   - If the scheduler has no record of this container ever running then assume the state is unknown.
func (api *API) findOrphans() {
	type orphankey struct {
		namespace string
		pipeline  string
		run       int64
	}

	// Collect all events.
	events := api.events.GetAll(false)
	orphanedRuns := map[orphankey]struct{}{}

	// Search events for any orphan runs.
	for event := range events {
		switch event.Kind {
		case models.EventKindStartedRun:
			// TODO(clintjedwards): This causes the data race alert to be angry,
			// but in theory it should be fine as we only read and write from
			// the var once. Need to find a way to pass trait objects without
			// Go complaining that other things can access them.
			evt, ok := event.Details.(*models.EventStartedRun)
			if !ok {
				log.Error().Interface("event", event).Msg("could not decode event into correct type")
				continue
			}

			_, exists := orphanedRuns[orphankey{
				namespace: evt.NamespaceID,
				pipeline:  evt.PipelineID,
				run:       evt.RunID,
			}]

			if !exists {
				orphanedRuns[orphankey{
					namespace: evt.NamespaceID,
					pipeline:  evt.PipelineID,
					run:       evt.RunID,
				}] = struct{}{}
			}

		case models.EventKindCompletedRun:
			evt, ok := event.Details.(*models.EventCompletedRun)
			if !ok {
				log.Error().Interface("event", event).Msg("could not decode event into correct type")
				continue
			}

			_, exists := orphanedRuns[orphankey{
				namespace: evt.NamespaceID,
				pipeline:  evt.PipelineID,
				run:       evt.RunID,
			}]

			if exists {
				delete(orphanedRuns, orphankey{
					namespace: evt.NamespaceID,
					pipeline:  evt.PipelineID,
					run:       evt.RunID,
				})
			}
		}
	}

	for orphan := range orphanedRuns {
		log.Info().Str("namespace", orphan.namespace).Str("pipeline", orphan.pipeline).
			Int64("run", orphan.run).Msg("attempting to complete orphaned run")

		err := api.repairOrphanRun(orphan.namespace, orphan.pipeline, orphan.run)
		if err != nil {
			log.Error().Err(err).Str("namespace", orphan.namespace).
				Str("pipeline", orphan.pipeline).Int64("run", orphan.run).Msg("could not repair orphan run")
		}
	}
}

// repairOrphanRun allows gofer to repair runs that are orphaned from a bug of sudden shutdown.
//
//   - If the run is unfinished: Attach the goroutine responsible for monitoring said run.
//   - If the container/task run is still running: Attach state watcher goroutine, truncate logs, attach new log watcher.
//   - If the container is in a finished state: Remove from run cache -> update container state -> clear out logs
//     -> update logs with new logs.
//   - If the scheduler has no record of this container ever running then assume the state is unknown.
func (api *API) repairOrphanRun(namespace, pipelineID string, runID int64) error {
	pipeline, err := api.db.GetPipeline(nil, namespace, pipelineID)
	if err != nil {
		return err
	}

	run, err := api.db.GetRun(namespace, pipelineID, runID)
	if err != nil {
		return err
	}

	runStateMachine := api.newRunStateMachine(&pipeline, &run)

	// For each run we also need to handle the individual task runs.
	for _, taskrunID := range run.TaskRuns {
		taskrun, err := api.db.GetTaskRun(run.Namespace, run.Pipeline, run.ID, taskrunID)
		if err != nil {
			log.Error().Err(err).Str("pipeline", run.Pipeline).Int64("run", run.ID).
				Msg("could not get run status for repair orphan")
			continue
		}

		if taskrun.State == models.TaskRunStateComplete {
			runStateMachine.TaskRuns.Set(taskrun.Task.GetID(), taskrun)
			continue
		}

		// If the taskrun was waiting to be scheduled then we have to make sure it gets scheduled as normal.
		if taskrun.State == models.TaskRunStateWaiting || taskrun.State == models.TaskRunStateProcessing {
			go runStateMachine.launchTaskRun(taskrun.Task)
			continue
		}

		// If it is unfinished and just need to be tracked then we just add log/state trackers onto it.
		go runStateMachine.handleLogUpdates(taskContainerID(taskrun.Namespace, taskrun.Pipeline, taskrun.Run, taskrun.ID), taskrun.ID)
		go func() {
			err = runStateMachine.waitTaskRunFinish(taskContainerID(taskrun.Namespace, taskrun.Pipeline, taskrun.Run, taskrun.ID), taskrun.ID)
			if err != nil {
				log.Error().Err(err).Str("task", taskrun.ID).
					Str("pipeline", taskrun.Pipeline).
					Int64("run", taskrun.Run).Msg("could not get state for container update")
			}
		}()
	}

	// If run is unfinished then we need to launch a goroutine to track its state.
	if run.State != models.RunStateComplete {
		go runStateMachine.waitRunFinish()
	}

	return nil
}

// createGRPCServer creates the gofer grpc server with all the proper settings; TLS enabled.
func (api *API) createGRPCServer() (*grpc.Server, error) {
	tlsConfig, err := api.generateTLSConfig(api.config.Server.TLSCertPath, api.config.Server.TLSKeyPath)
	if err != nil {
		return nil, err
	}

	panicHandler := func(p interface{}) (err error) {
		log.Error().Err(err).Interface("panic", p).Bytes("stack", debug.Stack()).Msg("server has encountered a fatal error")
		return status.Errorf(codes.Unknown, "server has encountered a fatal error and could not process request")
	}

	grpcServer := grpc.NewServer(
		// recovery should always be first
		grpc.UnaryInterceptor(
			grpc_middleware.ChainUnaryServer(
				grpc_recovery.UnaryServerInterceptor(grpc_recovery.WithRecoveryHandler(panicHandler)),
				grpc_auth.UnaryServerInterceptor(api.authenticate),
			),
		),
		grpc.StreamInterceptor(
			grpc_middleware.ChainStreamServer(
				grpc_recovery.StreamServerInterceptor(grpc_recovery.WithRecoveryHandler(panicHandler)),
				grpc_auth.StreamServerInterceptor(api.authenticate),
			),
		),

		// Handle TLS
		grpc.Creds(credentials.NewTLS(tlsConfig)),
	)

	reflection.Register(grpcServer)
	proto.RegisterGoferServer(grpcServer, api)

	return grpcServer, nil
}

// grpcDial establishes a connection with the request URL via GRPC.
func grpcDial(url string) (*grpc.ClientConn, error) {
	host, port, ok := strings.Cut(url, ":")
	if !ok {
		return nil, fmt.Errorf("could not parse url %q; format should be <host>:<port>", url)
	}

	var opt []grpc.DialOption
	var tlsConf *tls.Config

	// If we're testing in development bypass the cert checks.
	if host == "localhost" || host == "127.0.0.1" {
		tlsConf = &tls.Config{
			InsecureSkipVerify: true,
		}
		opt = append(opt, grpc.WithTransportCredentials(credentials.NewTLS(tlsConf)))
	}

	opt = append(opt, grpc.WithUnaryInterceptor(grpc_retry.UnaryClientInterceptor(grpc_retry.WithMax(3), grpc_retry.WithBackoff(grpc_retry.BackoffExponential(time.Millisecond*100)))))

	conn, err := grpc.Dial(fmt.Sprintf("%s:%s", host, port), opt...)
	if err != nil {
		return nil, fmt.Errorf("could not connect to server: %w", err)
	}

	return conn, nil
}
