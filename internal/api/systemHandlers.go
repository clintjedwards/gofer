package api

import (
	"context"
	"net/http"
	"strings"

	"github.com/danielgtaylor/huma/v2"

	"github.com/rs/zerolog/log"
	"google.golang.org/grpc/codes"
	"google.golang.org/grpc/status"
)

var appVersion = "0.0.dev_000000"

func parseVersion(versionString string) (version, commit string) {
	version, commit, err := strings.Cut(versionString, "_")
	if !err {
		return "", ""
	}

	return
}

func contains(s []string, e string) bool {
	for _, a := range s {
		if a == e {
			return true
		}
	}
	return false
}

type (
	DescribeSystemInfoRequest  struct{}
	DescribeSystemInfoResponse struct {
		Body struct {
			Commit string `json:"commit" example:"e83adcd" doc:"The commit of the current build"`
			Semver string `json:"semver" example:"1.0.0" doc:"The semver version of the current build"`
		}
	}
)

func (apictx *APIContext) registerDescribeSystemInfo(apiDesc huma.API) {
	// Description //
	huma.Register(apiDesc, huma.Operation{
		OperationID: "DescribeSystemInfo",
		Method:      http.MethodGet,
		Path:        "/api/system/info",
		Summary:     "Describe current system information",
		Description: "Return a number of internal meta information about the Gofer server.",
		Tags:        []string{"System"},
		// Handler //
	}, func(_ context.Context, _ *DescribeSystemInfoRequest) (*DescribeSystemInfoResponse, error) {
		version, commit := parseVersion(appVersion)
		resp := &DescribeSystemInfoResponse{}
		resp.Body.Commit = commit
		resp.Body.Semver = version

		return resp, nil
	})
}

type (
	DescribeSystemSummaryRequest  struct{}
	DescribeSystemSummaryResponse struct {
		Body struct {
			Namespaces         []string `json:"namespaces" example:"[\"default\",\"infra\",\"backend\",\"frontend\"]" doc:"List of all namespaces"`
			PipelineCount      int64    `json:"pipeline_count" example:"4" doc:"The number of pipelines registered"`
			RunCount           int64    `json:"run_count" example:"3" doc:"The number of runs completed"`
			TaskExecutionCount int64    `json:"task_execution_count" example:"7" doc:"The number of task executions completed"`
		}
	}
)

func (apictx *APIContext) registerDescribeSystemSummary(apiDesc huma.API) {
	// Description //
	huma.Register(apiDesc, huma.Operation{
		OperationID: "DescribeSystemSummary",
		Method:      http.MethodGet,
		Path:        "/api/system/summary",
		Summary:     "Describe various aspects about Gofer's current workloads",
		Description: "A general endpoint to retrieve various metrics about the Gofer service.",
		Tags:        []string{"System"},
		// Handler //
	}, func(_ context.Context, _ *DescribeSystemSummaryRequest) (*DescribeSystemSummaryResponse, error) {
		storedNamespaces, err := apictx.db.ListNamespaces(apictx.db, 0, 0)
		if err != nil {
			log.Error().Err(err).Msg("could not list namespaces")
			return nil, status.Errorf(codes.Internal, "could not list namespaces: %v", err)
		}

		namespaces := []string{}
		for _, namespace := range storedNamespaces {
			namespaces = append(namespaces, namespace.Name)
		}

		pipelineCount, err := apictx.db.GetPipelineCount(apictx.db)
		if err != nil {
			log.Error().Err(err).Msg("could not query for pipeline count")
			return nil, status.Errorf(codes.Internal, "could not query for pipeline count: %v", err)
		}

		runCount, err := apictx.db.GetPipelineRunsCount(apictx.db)
		if err != nil {
			log.Error().Err(err).Msg("could not query for pipeline run count")
			return nil, status.Errorf(codes.Internal, "could not query for pipeline run count: %v", err)
		}

		taskExecutionCount, err := apictx.db.GetPipelineTasksCount(apictx.db)
		if err != nil {
			log.Error().Err(err).Msg("could not query for pipeline task count")
			return nil, status.Errorf(codes.Internal, "could not query for pipeline task count: %v", err)
		}

		resp := &DescribeSystemSummaryResponse{}
		resp.Body.Namespaces = namespaces
		resp.Body.PipelineCount = pipelineCount
		resp.Body.RunCount = runCount
		resp.Body.TaskExecutionCount = taskExecutionCount

		return resp, nil
	})
}

type (
	ToggleEventIngressRequest struct {
		Auth string `header:"Authorization" example:"Bearer <your_api_token>" required:"true"`
	}
	ToggleEventIngressResponse struct {
		Body struct {
			Value bool `json:"value" example:"true" doc:"The current value for the boolean that controls event ingress"`
		}
	}
)

func (apictx *APIContext) registerToggleEventIngress(apiDesc huma.API) {
	// Description //
	huma.Register(apiDesc, huma.Operation{
		OperationID: "ToggleEventIngress",
		Method:      http.MethodPost,
		Path:        "/api/system/toggle-event-ingress",
		Summary:     "Toggle the ability for users to trigger pIpelines",
		Description: "Allows the admin to start or stop the execution of all pipelines within Gofer. This can be " +
			"useful under some security implications or for the purposes of defining general downtime and service maintenance.",
		Tags: []string{"System"},
		// Handler //
	}, func(ctx context.Context, _ *ToggleEventIngressRequest) (*ToggleEventIngressResponse, error) {
		if !isManagementUser(ctx) {
			return &ToggleEventIngressResponse{}, huma.NewError(http.StatusUnauthorized, "management token required for this action")
		}

		if !apictx.ignorePipelineRunEvents.CompareAndSwap(false, true) {
			apictx.ignorePipelineRunEvents.Store(false)
		}

		resp := &ToggleEventIngressResponse{}
		resp.Body.Value = apictx.ignorePipelineRunEvents.Load()

		return resp, nil
	})
}

type RepairOrphanRequest struct {
	Auth string `header:"Authorization" example:"Bearer <your_api_token>" required:"true"`
	Body struct {
		NamespaceID string `json:"namespace_id,omitempty" example:"default" default:"default" doc:"Unique identifier of the target namespace"`
		PipelineID  string `json:"pipeline_id" example:"simple_pipeline" doc:"Unique identifier for the target pipeline"`
		RunID       int64  `json:"run_id" example:"1" doc:"Unique identifier for the target run"`
	}
}

type RepairOrphanResponse struct{}

func (apictx *APIContext) registerRepairOrphan(apiDesc huma.API) {
	// Description //
	huma.Register(apiDesc, huma.Operation{
		OperationID: "RepairOrphan",
		Method:      http.MethodPost,
		Path:        "/api/system/repair-orphan",
		Summary:     "Manually attempt to repair an incomplete run",
		Description: "RepairOrphan is used when a single run has gotten into a state that does not reflect what actually " +
			"happened to the run. This can happen if the Gofer service crashes for unforeseen reasons. Usually this route is not " +
			"needed as Gofer will attempt  to resolve all orphaned runs upon startup. But in the rare case that a run gets " +
			"into a bad state during the service's normal execution this route can be used to attempt to repair the orphaned " +
			"run or at the very least mark it as failed so it isn't stuck in a unfinished state.",
		Tags: []string{"System"},
		// Handler //
	}, func(ctx context.Context, request *RepairOrphanRequest) (*RepairOrphanResponse, error) {
		if !isManagementUser(ctx) {
			return &RepairOrphanResponse{}, huma.NewError(http.StatusUnauthorized, "management token required for this action")
		}

		if request.Body.PipelineID == "" {
			return &RepairOrphanResponse{}, huma.NewError(http.StatusBadRequest, "pipeline_id required")
		}

		err := apictx.repairOrphanRun(request.Body.NamespaceID, request.Body.PipelineID, request.Body.RunID)
		if err != nil {
			return &RepairOrphanResponse{}, huma.NewError(http.StatusInternalServerError, "could not repair orphan run", err)
		}

		return &RepairOrphanResponse{}, nil
	})
}
