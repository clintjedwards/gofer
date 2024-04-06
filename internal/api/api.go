// Package api controls the bulk of the Gofer API logic.
package api

import (
	"context"
	"errors"
	"fmt"
	"net/http"
	"os"
	"os/signal"
	"sync/atomic"
	"syscall"
	"time"

	"github.com/clintjedwards/gofer/events"
	"github.com/clintjedwards/gofer/internal/config"
	"github.com/clintjedwards/gofer/internal/eventbus"
	"github.com/clintjedwards/gofer/internal/frontend"
	"github.com/clintjedwards/gofer/internal/models"
	"github.com/clintjedwards/gofer/internal/objectStore"
	"github.com/clintjedwards/gofer/internal/scheduler"
	"github.com/clintjedwards/gofer/internal/secretStore"
	"github.com/clintjedwards/gofer/internal/storage"
	"github.com/clintjedwards/gofer/internal/syncmap"
	"github.com/danielgtaylor/huma/v2"
	"github.com/danielgtaylor/huma/v2/adapters/humago"
	"github.com/go-chi/chi/v5/middleware"
	"github.com/rs/zerolog/log"
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

	// ErrNoValidConfiguration is returned there is no pipeline configuration that is avaiable for use.
	ErrNoValidConfiguration = errors.New("api: there was no valid, live pipeline configuration found")

	// ErrExtensionNotFound is returned when a pipeline configuration contains a extension that was not registered with the API.
	ErrExtensionNotFound = errors.New("api: extension is not found")
)

type CancelContext struct {
	ctx    context.Context
	cancel context.CancelFunc
}

// APIContext represents the main Gofer service APIContext. It is run using a HTTP combined server.
// This main APIContext handles 99% of interactions with the gofer service itself and is only missing the hooks for the
// gofer events service.
//
//revive:disable-next-line (This exception exists because we don't care about the stutter here)
type APIContext struct {
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
	// Extensions is an in-memory map of currently registered extensions. These extensions are registered on startup and
	// launched as long running containers via the scheduler. Gofer refers to this cache as a way to communicate
	// quickly with the containers and their potentially changing endpoints.
	extensions syncmap.Syncmap[string, *models.Extension]

	// ignorePipelineRunEvents controls if pipelines can extension runs globally. If this is set to false the entire Gofer
	// service will not schedule new runs.
	ignorePipelineRunEvents *atomic.Bool

	// events acts as an event bus for the Gofer application. It is used throughout the whole application to give
	// different parts of the application the ability to listen for and respond to events that might happen in other
	// parts.
	events *eventbus.EventBus
}

// NewAPI creates a new instance of the main Gofer API service.
func NewAPI(config *config.API, storage storage.DB, scheduler scheduler.Engine, objectStore objectStore.Engine,
	secretStore secretStore.Engine,
) (*APIContext, error) {
	eventbus, err := eventbus.New(storage, config.EventLogRetention, config.EventPruneInterval)
	if err != nil {
		return nil, fmt.Errorf("could not init event bus: %w", err)
	}

	ctx, cancel := context.WithCancel(context.Background())

	var ignorePipelineRunEvents atomic.Bool
	ignorePipelineRunEvents.Store(config.IgnorePipelineRunEvents)

	newAPI := &APIContext{
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
		extensions:              syncmap.New[string, *models.Extension](),
	}

	err = newAPI.createDefaultNamespace()
	if err != nil {
		return nil, fmt.Errorf("could not create default namespace: %w", err)
	}

	err = newAPI.installBaseExtensions()
	if err != nil {
		return nil, fmt.Errorf("could not install base extensions: %w", err)
	}

	err = newAPI.startExtensions()
	if err != nil {
		return nil, fmt.Errorf("could not start extensions: %w", err)
	}

	err = newAPI.restoreExtensionSubscriptions()
	if err != nil {
		newAPI.cleanup()
		return nil, fmt.Errorf("could not restore extension subscriptions: %w", err)
	}

	// findOrphans is a repair method that picks up where the gofer service left off if it was shutdown while
	// a run was currently in progress.
	go newAPI.findOrphans()

	return newAPI, nil
}

// cleanup gracefully cleans up all goroutines to ensure a clean shutdown.
func (apictx *APIContext) cleanup() {
	apictx.ignorePipelineRunEvents.Store(true)

	// Send graceful stop to all extensions
	apictx.stopExtensions()

	// Stop all goroutines which should stop the event processing pipeline and the extension monitoring.
	apictx.context.cancel()
}

// StartAPIService starts the Gofer API service and blocks until a SIGINT or SIGTERM is received.
func (apictx *APIContext) StartAPIService() {
	tlsConfig, err := apictx.generateTLSConfig(apictx.config.Server.TLSCertPath, apictx.config.Server.TLSKeyPath)
	if err != nil {
		log.Fatal().Err(err).Msg("could not get proper TLS config")
	}

	// Assign all routes and handlers
	router, _ := InitRouter(apictx)

	httpServer := http.Server{
		Addr:         apictx.config.Server.ListenAddress,
		Handler:      loggingMiddleware(router),
		WriteTimeout: apictx.config.Server.WriteTimeout,
		ReadTimeout:  apictx.config.Server.ReadTimeout,
		IdleTimeout:  apictx.config.Server.IdleTimeout,
		TLSConfig:    tlsConfig,
	}

	// Run our server in a goroutine and listen for signals that indicate graceful shutdown
	go func() {
		if err := httpServer.ListenAndServeTLS("", ""); err != nil && err != http.ErrServerClosed {
			log.Fatal().Err(err).Msg("server exited abnormally")
		}
	}()
	log.Info().Str("url", apictx.config.Server.ListenAddress).Msg("started gofer http service")

	c := make(chan os.Signal, 1)
	signal.Notify(c, syscall.SIGTERM, syscall.SIGINT)
	<-c

	// On ctrl-c we need to clean up not only the connections from the server, but make sure all the currently
	// running jobs are logged and exited properly.
	apictx.cleanup()

	// Doesn't block if no connections, otherwise will wait until the timeout deadline or connections to finish,
	// whichever comes first.
	ctx, cancel := context.WithTimeout(context.Background(), apictx.config.Server.ShutdownTimeout) // shutdown gracefully
	defer cancel()

	err = httpServer.Shutdown(ctx)
	if err != nil {
		log.Error().Err(err).Msg("could not shutdown server in timeout specified")
		return
	}

	log.Info().Msg("http server exited gracefully")
}

// The logging middleware has to be run before the final call to return the request.
// This is because we wrap the responseWriter to gain information from it after it
// has been written to (this enables us to get things that we only know after the request has been served like status codes).
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
			Float64("elapsed_ms", float64(time.Since(start))/float64(time.Millisecond)).
			Msg("")
	})
}

// Create a new http router that gets populated by huma lib. Huma helps create an OpenAPI spec and documentation
// from REST code. We export this function so that we can use it in external scripts to generate the OpenAPI spec
// for this API in other places.
func InitRouter(apictx *APIContext) (router *http.ServeMux, apiDescription huma.API) {
	router = http.NewServeMux()

	version, _ := parseVersion(appVersion)
	humaConfig := huma.DefaultConfig("Gofer", version)
	humaConfig.Info.Description = "Gofer is an opinionated, streamlined automation engine designed for the cloud-native " +
		"era. It specializes in executing your custom scripts in a containerized environment, making it versatile for " +
		"both developers and operations teams. Deploy Gofer effortlessly as a single static binary, and " +
		"manage it using expressive, declarative configurations written in real programming languages. Once " +
		"set up, Gofer takes care of scheduling and running your automation tasks—be it on Nomad, Kubernetes, or even Local Docker." +
		"\n" +
		"Its primary function is to execute short-term jobs like code linting, build automation, testing, port scanning, " +
		"ETL operations, or any task you can containerize and trigger based on events."

	humaConfig.DocsPath = "/api/docs"
	humaConfig.OpenAPIPath = "/api/docs/openapi"
	humaConfig.Servers = append(humaConfig.Servers, &huma.Server{
		URL: apictx.config.Server.Address,
	})
	humaConfig.Components.SecuritySchemes = map[string]*huma.SecurityScheme{
		"bearer": {
			Type:   "http",
			Scheme: "bearer",
		},
	}

	apiDescription = humago.New(router, humaConfig)
	apiDescription.UseMiddleware(authMiddleware(apictx, apiDescription))

	/* /api/system */
	apictx.registerDescribeSystemInfo(apiDescription)
	apictx.registerDescribeSystemSummary(apiDescription)
	apictx.registerToggleEventIngress(apiDescription)
	apictx.registerRepairOrphan(apiDescription)

	/* /api/tokens */
	apictx.registerCreateToken(apiDescription)
	apictx.registerListTokens(apiDescription)
	apictx.registerDescribeTokenByID(apiDescription)
	apictx.registerDescribeTokenByHash(apiDescription)
	apictx.registerEnableToken(apiDescription)
	apictx.registerDisableToken(apiDescription)
	apictx.registerDeleteToken(apiDescription)
	apictx.registerCreateBootstrapToken(apiDescription)

	/* /api/pipelines/{pipeline_id}/runs/{run_id}/tasks/{task_execution_id} */
	apictx.registerDescribeTaskExecution(apiDescription)
	apictx.registerListTaskExecutions(apiDescription)
	apictx.registerCancelTaskExecution(apiDescription)
	router.HandleFunc("/api/pipelines/{pipeline_id}/runs/{run_id}/tasks/{task_execution_id}/attach",
		apictx.attachToTaskExecutionHandler)

	// Set up the frontend paths last since they capture everything that isn't in the API path.
	if apictx.config.Development.LoadFrontendFilesFromDisk {
		log.Warn().Msg("Loading frontend files from local disk dir 'public'; Not for use in production.")
		router.Handle("/", frontend.LocalHandler())
	} else {
		router.Handle("/", frontend.StaticHandler())
	}

	if apictx.config.Development.GenerateOpenAPISpecFiles {
		generateOpenAPIFiles(apiDescription)
	}

	return router, apiDescription
}

// Generates OpenAPI Yaml files that other services can use to generate code for Gofer's API.
func generateOpenAPIFiles(apiDescription huma.API) {
	output, err := apiDescription.OpenAPI().YAML()
	if err != nil {
		panic(err)
	}

	file, err := os.Create("openapi.yaml")
	if err != nil {
		panic(err)
	}
	defer file.Close()

	_, err = file.Write(output)
	if err != nil {
		panic(err)
	}
}

func (apictx *APIContext) installBaseExtensions() error {
	// if !apictx.config.Extensions.InstallBaseExtensions {
	// 	return nil
	// }

	// registeredExtensions, err := apictx.db.ListGlobalExtensionRegistrations(apictx.db, 0, 0)
	// if err != nil {
	// 	return err
	// }

	// cronInstalled := false
	// intervalInstalled := false

	// for _, extension := range registeredExtensions {
	// 	if strings.EqualFold(extension.ID, "cron") {
	// 		cronInstalled = true
	// 	}

	// 	if strings.EqualFold(extension.ID, "interval") {
	// 		intervalInstalled = true
	// 	}
	// }

	// if !cronInstalled {
	// 	registration := models.ExtensionRegistration{}
	// 	registration.FromInstallExtensionRequest(&proto.InstallExtensionRequest{
	// 		Name:  "cron",
	// 		Image: "ghcr.io/clintjedwards/gofer/extensions/cron:latest",
	// 	})

	// 	err := apictx.db.InsertGlobalExtensionRegistration(apictx.db, registration.ToStorage())
	// 	if err != nil {
	// 		if !errors.Is(err, storage.ErrEntityExists) {
	// 			return err
	// 		}
	// 	}

	// 	log.Info().Str("name", registration.Name).Str("image", registration.Image).
	// 		Msg("registered base extension automatically due to 'install_base_extensions' config")
	// }

	// if !intervalInstalled {
	// 	registration := models.ExtensionRegistration{}
	// 	registration.FromInstallExtensionRequest(&proto.InstallExtensionRequest{
	// 		Name:  "interval",
	// 		Image: "ghcr.io/clintjedwards/gofer/extensions/interval:latest",
	// 	})

	// 	err := apictx.db.InsertGlobalExtensionRegistration(apictx.db, registration.ToStorage())
	// 	if err != nil {
	// 		if !errors.Is(err, storage.ErrEntityExists) {
	// 			return err
	// 		}
	// 	}

	// 	log.Info().Str("name", registration.Name).Str("image", registration.Image).
	// 		Msg("registered base extension automatically due to 'install_base_extensions' config")
	// }

	return nil
}

// Gofer starts with a default namespace that all users have access to.
func (apictx *APIContext) createDefaultNamespace() error {
	namespace := models.NewNamespace("default", "Default", "default namespace")
	err := apictx.db.InsertNamespace(apictx.db, namespace.ToStorage())
	if err != nil {
		if errors.Is(err, storage.ErrEntityExists) {
			return nil
		}

		return err
	}

	apictx.events.Publish(events.EventNamespaceCreated{
		NamespaceID: namespace.ID,
	})

	return nil
}

// findOrphans allows the gofer service to be shutdown and still pick back up where it left off on next startup.
// It does this by simply re-attaching the state monitoring go routines for a run and its child task runs.
// While simple on its face this is actually quite non-trivial as it requires delicate figuring out where the run is
// currently in its lifecycle and accounting for any state it could possibly be in.
//
// Gofer identifies runs that haven't fully completed by searching through and matching run events.
// If an event is missing it's "Completed" event then on startup Gofer considers that run not finished and attempts
// to recover it.
//
// It then asks the scheduler for the last status of the container and appropriately either:
//   - If the run is unfinished: Attach the goroutine responsible for monitoring said run.
//   - If the container/task run is still running: Attach state watcher goroutine, truncate logs, attach new log watcher.
//   - If the container is in a finished state: Remove from run cache -> update container state -> clear out logs
//     -> update logs with new logs.
//   - If the scheduler has no record of this container ever running then assume the state is unknown.
func (apictx *APIContext) findOrphans() {
	type orphankey struct {
		namespace string
		pipeline  string
		run       int64
	}

	// Collect all eventList.
	eventList := apictx.events.GetAll(false)
	orphanedRuns := map[orphankey]struct{}{}

	// Search events for any orphan runs.
	for event := range eventList {
		switch event.Type {
		case events.EventTypeRunStarted:
			// This causes the data race alert to be angry,
			// but in theory it should be fine as we only read and write from
			// the var once. Need to find a way to pass trait objects without
			// Go complaining that other things can access them.
			evt, ok := event.Details.(*events.EventRunStarted)
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

		case events.EventTypeRunCompleted:
			evt, ok := event.Details.(*events.EventRunCompleted)
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

		err := apictx.repairOrphanRun(orphan.namespace, orphan.pipeline, orphan.run)
		if err != nil {
			log.Error().Err(err).Str("namespace", orphan.namespace).
				Str("pipeline", orphan.pipeline).Int64("run", orphan.run).Msg("could not repair orphan run")
		}
	}
}

// repairOrphanRun allows gofer to repair runs that are orphaned from a loss of tracking or sudden shutdown.
//
//   - If the run is unfinished: Attach the goroutine responsible for monitoring said run.
//   - If the container/task run is still running: Attach state watcher goroutine, truncate logs, attach new log watcher.
//   - If the container is in a finished state: update container state -> clear out logs
//     -> update logs with new logs.
//   - If the scheduler has no record of this container ever running then assume the state is unknown.
func (apictx *APIContext) repairOrphanRun(namespaceID, pipelineID string, runID int64) error {
	// metadataRaw, err := apictx.db.GetPipelineMetadata(apictx.db, namespaceID, pipelineID)
	// if err != nil {
	// 	return err
	// }

	// var metadata models.PipelineMetadata
	// metadata.FromStorage(&metadataRaw)

	// latestConfigRaw, err := apictx.db.GetLatestLivePipelineConfig(apictx.db, namespaceID, pipelineID)
	// if err != nil {
	// 	return err
	// }

	// tasksRaw, err := apictx.db.ListPipelineTasks(apictx.db, namespaceID, pipelineID, latestConfigRaw.Version)
	// if err != nil {
	// 	return err
	// }

	// var latestConfig models.PipelineConfig
	// latestConfig.FromStorage(&latestConfigRaw, &tasksRaw)

	// runRaw, err := apictx.db.GetPipelineRun(apictx.db, namespaceID, pipelineID, runID)
	// if err != nil {
	// 	return err
	// }

	// var run models.Run
	// run.FromStorage(&runRaw)

	// taskExecutionsRaw, err := apictx.db.ListPipelineTaskExecutions(apictx.db, 0, 0, namespaceID, pipelineID, runID)
	// if err != nil {
	// 	return err
	// }

	// var taskExecutions []models.TaskExecution
	// for _, taskExecutionRaw := range taskExecutionsRaw {
	// 	var taskExecution models.TaskExecution
	// 	taskExecution.FromStorage(&taskExecutionRaw)
	// 	taskExecutions = append(taskExecutions, taskExecution)
	// }

	// // In order to manage the orphaned run we will create a new state machine and make it part of that.
	// runStateMachine := apictx.newRunStateMachine(&metadata, &latestConfig, &run)

	// // For each run we also need to evaluate the individual task runs.
	// for _, taskexecution := range taskExecutions {
	// 	taskExecution := taskexecution

	// 	// If the task run was actually marked complete in the database. Then we add it to the state machine.
	// 	// This is necessary because eventually we will compute whether the run was complete and we'll need the
	// 	// state of that run.
	// 	if taskExecution.State == models.TaskExecutionStateComplete {
	// 		runStateMachine.TaskExecutions.Set(taskExecution.Task.ID, taskExecution)
	// 		continue
	// 	}

	// 	// If the taskexecution was waiting to be scheduled then we have to make sure it gets scheduled as normal.
	// 	if taskExecution.State == models.TaskExecutionStateWaiting || taskExecution.State == models.TaskExecutionStateProcessing {
	// 		go runStateMachine.launchTaskExecution(taskExecution.Task, false)
	// 		continue
	// 	}

	// 	// If the task run was in a state where it had been launched and just needs to be tracked then we just
	// 	// add log/state trackers onto it.
	// 	runStateMachine.TaskExecutions.Set(taskExecution.Task.ID, taskExecution)
	// 	go runStateMachine.handleLogUpdates(taskContainerID(taskExecution.Namespace, taskExecution.Pipeline, taskExecution.Run, taskExecution.ID), taskExecution.ID)
	// 	go func() {
	// 		err = runStateMachine.waitTaskExecutionFinish(taskContainerID(taskExecution.Namespace, taskExecution.Pipeline, taskExecution.Run, taskExecution.ID), taskExecution.ID)
	// 		if err != nil {
	// 			log.Error().Err(err).Str("task", taskExecution.ID).
	// 				Str("pipeline", taskExecution.Pipeline).
	// 				Int64("run", taskExecution.Run).Msg("could not get state for container update")
	// 		}
	// 	}()
	// }

	// // If run is unfinished then we need to launch a goroutine to track its state.
	// if run.State != models.RunStateComplete {
	// 	go runStateMachine.waitRunFinish()
	// }

	return nil
}

type VariableSource string

const (
	VariableSourceUnknown        VariableSource = "UNKNOWN"
	VariableSourcePipelineConfig VariableSource = "PIPELINE_CONFIG"
	VariableSourceSystem         VariableSource = "SYSTEM"
	VariableSourceRunOptions     VariableSource = "RUN_OPTIONS"
	VariableSourceExtension      VariableSource = "EXTENSION"
)

// A variable is a key value pair that is used either in a run or task level.
// The variable is inserted as an environment variable to an eventual task run.
// It can be owned by different parts of the system which control where
// the potentially sensitive variables might show up.
type Variable struct {
	Key    string         `json:"key" example:"MYAPP_VAR_ONE" doc:"The key of the environment variable"`
	Value  string         `json:"value" example:"some_value" doc:"The value of the environment variable"`
	Source VariableSource `json:"source" example:"PIPELINE_CONFIG" doc:"Where the variable originated"`
}

type RegistryAuth struct {
	User string `json:"user" example:"some_user" doc:"The username for image registry auth"`
	Pass string `json:"pass" example:"hunter2" doc:"The password for the image registry auth"`
}
