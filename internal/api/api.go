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
	"sync"
	"syscall"
	"time"

	"github.com/clintjedwards/gofer/internal/config"
	"github.com/clintjedwards/gofer/internal/eventbus"
	"github.com/clintjedwards/gofer/internal/models"
	"github.com/clintjedwards/gofer/internal/objectStore"
	"github.com/clintjedwards/gofer/internal/scheduler"
	"github.com/clintjedwards/gofer/internal/secretStore"
	"github.com/clintjedwards/gofer/internal/storage"
	"github.com/clintjedwards/gofer/proto"
	"github.com/gorilla/handlers"
	"github.com/gorilla/mux"
	grpc_middleware "github.com/grpc-ecosystem/go-grpc-middleware"
	grpc_auth "github.com/grpc-ecosystem/go-grpc-middleware/auth"
	grpc_recovery "github.com/grpc-ecosystem/go-grpc-middleware/recovery"
	grpc_retry "github.com/grpc-ecosystem/go-grpc-middleware/retry"
	"github.com/improbable-eng/grpc-web/go/grpcweb"
	"github.com/rs/zerolog/log"
	"go.uber.org/atomic"
	"google.golang.org/grpc"
	"google.golang.org/grpc/codes"
	"google.golang.org/grpc/credentials"
	"google.golang.org/grpc/reflection"
	"google.golang.org/grpc/status"
)

var (
	// ErrPipelineNotActive is returned when a request is made against a pipeline that is not in the active state.
	ErrPipelineNotActive = errors.New("api: pipeline is not in state 'active'")

	// ErrPipelineActive is returned when a request is made against a pipeline in the active state.
	ErrPipelineActive = errors.New("api: pipeline is in state 'active'")

	// ErrPipelineAbandoned is returned when a request is made against a pipeline in the abandoned state.
	ErrPipelineAbandoned = errors.New("api: pipeline is in state 'abandoned'")

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
	storage storage.Engine

	// Scheduler is the mechanism in which Gofer uses to run its individual containers. It leverages that backend
	// scheduler to do most of the work on running the user's task runs(docker containers).
	scheduler scheduler.Engine

	// ObjectStore is the mechanism in which Gofer stores pipeline and run level objects. The implementation here
	// is meant to act as a basic object store.
	objectStore objectStore.Engine

	// SecretStore is the mechanism in which Gofer pipeline secrets. This is the way in which users can fill pipeline
	// files with secrets.
	secretStore secretStore.Engine

	// Triggers is an in-memory map of currently registered triggers. These triggers are registered on startup and
	// launched as long running containers via the scheduler. Gofer refers to this cache as a way to communicate
	// quickly with the containers and their potentially changing endpoints.
	triggers map[string]*models.Trigger

	// Notifiers is an in-memory map of the currently registered notifiers. These notifiers are registered on startup
	// and launched as needed at the end of a user's pipeline run. Gofer refers to this cache as a way to quickly look
	// up which container is needed to be launched.
	notifiers map[string]*models.Notifier

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
func NewAPI(config *config.API, storage storage.Engine, scheduler scheduler.Engine, objectStore objectStore.Engine, secretStore secretStore.Engine) (*API, error) {
	eventbus, err := eventbus.New(storage, config.EventLogRetention, config.PruneEventsInterval)
	if err != nil {
		return nil, fmt.Errorf("could not init event bus: %w", err)
	}

	ctx, cancel := context.WithCancel(context.Background())

	newAPI := &API{
		context: &CancelContext{
			ctx:    ctx,
			cancel: cancel,
		},
		config:                  config,
		storage:                 storage,
		events:                  eventbus,
		scheduler:               scheduler,
		objectStore:             objectStore,
		secretStore:             secretStore,
		ignorePipelineRunEvents: atomic.NewBool(config.IgnorePipelineRunEvents),
		triggers:                map[string]*models.Trigger{},
		notifiers:               map[string]*models.Notifier{},
	}

	err = newAPI.createDefaultNamespace()
	if err != nil {
		return nil, fmt.Errorf("could not create default namespace: %w", err)
	}

	// findOrphans is a repair method that picks up where the gofer service left off if it was shutdown while
	// a run was currently in progress.
	go newAPI.findOrphans()

	// Register notifiers with API so that they can be looked up easily.
	newAPI.registerNotifiers()

	err = newAPI.startTriggers()
	if err != nil {
		return nil, fmt.Errorf("could not start triggers: %w", err)
	}

	err = newAPI.restoreTriggerSubscriptions()
	if err != nil {
		newAPI.cleanup()
		return nil, fmt.Errorf("could not restore trigger subscriptions: %w", err)
	}

	// These two functions are responsible for gofer's trigger event loop system. The first launches goroutines that
	// consumes events from triggers and the latter processes them into pipeline runs.
	newAPI.checkForTriggerEvents(newAPI.context.ctx)
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

// Gofer starts with a default namespace that all users have access to.
func (api *API) createDefaultNamespace() error {
	namespace := models.NewNamespace(namespaceDefaultID, namespaceDefaultName, "default namespace")
	err := api.storage.AddNamespace(storage.AddNamespaceRequest{
		Namespace: namespace,
	})
	if err != nil {
		if errors.Is(err, storage.ErrEntityExists) {
			return nil
		}

		return err
	}

	api.events.Publish(models.NewEventCreatedNamespace(*namespace))

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
//   * If the run is unfinished: Attach the goroutine responsible for monitoring said run.
//   * If the container/task run is still running: Attach state watcher goroutine, truncate logs, attach new log watcher.
//   * If the container is in a finished state: Remove from run cache -> update container state -> clear out logs
//     -> update logs with new logs.
//   * If the scheduler has no record of this container ever running then assume the state is unknown.
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
		switch evt := event.(type) {
		case *models.EventStartedRun:
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

		case *models.EventCompletedRun:
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
//   * If the run is unfinished: Attach the goroutine responsible for monitoring said run.
//   * If the container/task run is still running: Attach state watcher goroutine, truncate logs, attach new log watcher.
//   * If the container is in a finished state: Remove from run cache -> update container state -> clear out logs
//     -> update logs with new logs.
//   * If the scheduler has no record of this container ever running then assume the state is unknown.
func (api *API) repairOrphanRun(namespace, pipeline string, runID int64) error {
	run, err := api.storage.GetRun(storage.GetRunRequest{
		NamespaceID: namespace,
		PipelineID:  pipeline,
		ID:          runID,
	})
	if err != nil {
		return err
	}

	var taskStatusMap sync.Map

	// For each run we also need to handle the individual task runs.
	for _, taskrunID := range run.TaskRuns {
		taskrun, err := api.storage.GetTaskRun(storage.GetTaskRunRequest{
			NamespaceID: run.NamespaceID,
			PipelineID:  run.PipelineID,
			RunID:       run.ID,
			ID:          taskrunID,
		})
		if err != nil {
			log.Error().Err(err).Str("pipeline", run.PipelineID).Int64("run", run.ID).
				Msg("could not get run status for repair orphan")
			continue
		}

		if taskrun.IsComplete() {
			taskStatusMap.Store(taskrun.Task.ID, taskrun.State)
			continue
		}

		// If the task run isn't complete we need to further investigate.
		//   * If its currently running then we need to attach a state monitor to it so we know when its actually
		//     finished.
		//   * If it is currently waiting to be run, then we need to "revive" it by setting it up to run when
		//     its dependencies have finished.
		// The absence of a schedulerID is only a problem if the container was marked as running.
		// If this is the case the container is lost and we just mark it as "unknown".
		if taskrun.SchedulerID == "" && taskrun.State == models.ContainerStateRunning {
			taskrun.SetFinishedAbnormal(models.ContainerStateUnknown, models.TaskRunFailure{
				Kind:        models.TaskRunFailureKindOrphaned,
				Description: "could not find schedulerID for taskrun during recovery.",
			}, 1)

			taskStatusMap.Store(taskrun.Task.ID, taskrun.State)

			err = api.storage.UpdateTaskRun(storage.UpdateTaskRunRequest{TaskRun: taskrun})
			if err != nil {
				log.Error().Err(err).Str("task", taskrun.ID).
					Str("pipeline", taskrun.PipelineID).
					Int64("run", taskrun.RunID).Msg("could not update task run state due to storage err")
			}
			continue
		}

		// If the taskrun was waiting to be scheduled then it will not have a schedulerID yet. As such we
		// need to make sure it gets scheduled as normal.
		if taskrun.State == models.ContainerStateWaiting || taskrun.State == models.ContainerStateProcessing {
			go api.reviveLostTaskRun(&taskStatusMap, taskrun)
			continue
		}

		// If it is unfinished and just need to be tracked then we just add log/state trackers onto it.
		go api.handleLogUpdates(taskrun.SchedulerID, taskrun)
		go func() {
			err = api.waitTaskRunFinish(taskrun.SchedulerID, taskrun)
			if err != nil {
				log.Error().Err(err).Str("task", taskrun.ID).
					Str("pipeline", taskrun.PipelineID).
					Int64("run", taskrun.RunID).Msg("could not get state for container update")
			}
			taskStatusMap.Store(taskrun.Task.ID, taskrun.State)
			err = api.storage.UpdateTaskRun(storage.UpdateTaskRunRequest{TaskRun: taskrun})
			if err != nil {
				log.Error().Err(err).Str("task", taskrun.ID).
					Str("pipeline", taskrun.PipelineID).
					Int64("run", taskrun.RunID).Msg("could not update task run state due to storage err")
			}
		}()
	}

	// If run is unfinished then we need to launch a goroutine to track its state.
	if !run.IsComplete() {
		go api.monitorRunStatus(run.NamespaceID, run.PipelineID, run.ID, &taskStatusMap) //nolint:errcheck
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
