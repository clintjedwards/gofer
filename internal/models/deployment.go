package models

import (
	"encoding/json"
	"time"

	"github.com/clintjedwards/gofer/internal/storage"
	proto "github.com/clintjedwards/gofer/proto/go"
	"github.com/rs/zerolog/log"
)

type DeploymentState string

const (
	DeploymentStateUnknown  DeploymentState = "UNKNOWN" // The state of the run is unknown.
	DeploymentStateRunning  DeploymentState = "RUNNING"
	DeploymentStateComplete DeploymentState = "COMPLETE"
)

type DeploymentStatus string

const (
	// Could not determine current state of the status. Should only be in this state if
	// the deployment has not yet completed.
	DeploymentStatusUnknown    DeploymentStatus = "UNKNOWN"
	DeploymentStatusFailed     DeploymentStatus = "FAILED"
	DeploymentStatusSuccessful DeploymentStatus = "SUCCESSFUL"
)

type DeploymentStatusReasonKind string

const (
	// Gofer has no idea how the deployment got into this state.
	DeploymentStatusReasonUnknown DeploymentStatusReasonKind = "UNKNOWN"
)

type DeploymentStatusReason struct {
	// The specific type of deployment failure. Good for documentation about what it might be.
	Reason      DeploymentStatusReasonKind `json:"kind"`
	Description string                     `json:"description"` // The description of why the run might have failed.
}

func (r *DeploymentStatusReason) ToJSON() string {
	reason, err := json.Marshal(r)
	if err != nil {
		log.Fatal().Err(err).Msg("failed to convert deployment status reason to json")
	}

	return string(reason)
}

func (r *DeploymentStatusReason) ToProto() *proto.DeploymentStatusReason {
	return &proto.DeploymentStatusReason{
		Reason:      proto.DeploymentStatusReason_DeploymentStatusReasonKind(proto.DeploymentStatusReason_DeploymentStatusReasonKind_value[string(r.Reason)]),
		Description: r.Description,
	}
}

// A deployment represents a transition between pipeline versions.
type Deployment struct {
	Namespace    string                  `json:"namespace"`     // Unique ID of namespace.
	Pipeline     string                  `json:"pipeline"`      // The unique ID of the related pipeline.
	ID           int64                   `json:"id"`            // Unique identifier for deployment
	StartVersion int64                   `json:"start_version"` // What version of the pipeline is being deprecated.
	EndVersion   int64                   `json:"end_version"`   // What version of the pipeline are we moving to.
	Started      int64                   `json:"started"`       // Time of run start in epoch milli.
	Ended        int64                   `json:"ended"`         // Time of run finish in epoch milli.
	State        DeploymentState         `json:"state"`         // The current state of the run.
	Status       DeploymentStatus        `json:"status"`        // The current status of the run.
	StatusReason *DeploymentStatusReason `json:"status_reason"` // Contains more information about a run's current status.
	Logs         []Event                 `json:"logs"`          // An ordered event stream of what happened during the deployment.
}

func NewDeployment(namespace, pipeline string, id, startVersion, endVersion int64) *Deployment {
	return &Deployment{
		Namespace:    namespace,
		Pipeline:     pipeline,
		ID:           id,
		StartVersion: startVersion,
		EndVersion:   endVersion,
		Started:      time.Now().UnixMilli(),
		Ended:        0,
		State:        DeploymentStateRunning,
		Status:       DeploymentStatusUnknown,
		StatusReason: nil,
		Logs:         []Event{},
	}
}

func (d *Deployment) ToProto() *proto.Deployment {
	var statusReason *proto.DeploymentStatusReason
	if d.StatusReason != nil {
		statusReason = d.StatusReason.ToProto()
	}

	events := []*proto.Event{}
	for _, event := range d.Logs {
		evt, _ := event.ToProto()
		events = append(events, evt)
	}

	return &proto.Deployment{
		Namespace:    d.Namespace,
		Pipeline:     d.Pipeline,
		Id:           d.ID,
		StartVersion: d.StartVersion,
		EndVersion:   d.EndVersion,
		Started:      d.Started,
		Ended:        d.Ended,
		State:        proto.Deployment_DeploymentState(proto.Deployment_DeploymentState_value[string(d.State)]),
		Status:       proto.Deployment_DeploymentStatus(proto.Deployment_DeploymentStatus_value[string(d.Status)]),
		StatusReason: statusReason,
		Logs:         events,
	}
}

func (d *Deployment) ToStorage() *storage.PipelineDeployment {
	events := []*storage.Event{}
	for _, event := range d.Logs {
		evt := event.ToStorage()
		events = append(events, evt)
	}

	logs, err := json.Marshal(events)
	if err != nil {
		log.Fatal().Err(err).Msg("could not marshal ToStorage for deployment")
	}

	return &storage.PipelineDeployment{
		Namespace:    d.Namespace,
		Pipeline:     d.Pipeline,
		ID:           d.ID,
		StartVersion: d.StartVersion,
		EndVersion:   d.EndVersion,
		Started:      d.Started,
		Ended:        d.Ended,
		State:        string(d.State),
		Status:       string(d.Status),
		StatusReason: d.StatusReason.ToJSON(),
		Logs:         string(logs),
	}
}

func (d *Deployment) FromStorage(storage *storage.PipelineDeployment) {
	var statusReason DeploymentStatusReason
	err := json.Unmarshal([]byte(storage.StatusReason), &statusReason)
	if err != nil {
		log.Fatal().Err(err).Msg("could not marshal ToStorage for deployment")
	}

	var logs []Event
	err = json.Unmarshal([]byte(storage.Logs), &logs)
	if err != nil {
		log.Fatal().Err(err).Msg("could not marshal ToStorage for deployment")
	}

	d.Namespace = storage.Namespace
	d.Pipeline = storage.Pipeline
	d.ID = storage.ID
	d.StartVersion = storage.StartVersion
	d.EndVersion = storage.EndVersion
	d.Started = storage.Started
	d.Ended = storage.Ended
	d.State = DeploymentState(storage.State)
	d.Status = DeploymentStatus(storage.Status)
	d.StatusReason = &statusReason
	d.Logs = logs
}
