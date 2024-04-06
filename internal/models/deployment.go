package models

import (
	"encoding/json"
	"fmt"
	"strconv"
	"time"

	"github.com/clintjedwards/gofer/internal/storage"
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
	Reason      DeploymentStatusReasonKind `json:"reason" example:"ABNORMAL_EXIT" doc:"Specific reason type; useful for documentation"`
	Description string                     `json:"description" example:"task exited without an error code of 0" doc:"A humanized description for what occurred"`
}

func (r *DeploymentStatusReason) ToJSON() string {
	reason, err := json.Marshal(r)
	if err != nil {
		log.Fatal().Err(err).Msg("failed to convert deployment status reason to json")
	}

	return string(reason)
}

// A deployment represents a transition between pipeline versions.
type Deployment struct {
	NamespaceID  string                  `json:"namespace_id" example:"default" doc:"Unique identifier of the target namespace"`
	PipelineID   string                  `json:"pipeline_id" example:"simple_pipeline" doc:"Unique identifier for the target pipeline"`
	DeploymentID int64                   `json:"deployment_id" example:"23" doc:"Unique identifier for the deployment"`
	StartVersion int64                   `json:"start_version" example:"1" doc:"What version of the pipeline is being deprecated"`
	EndVersion   int64                   `json:"end_version" example:"2" doc:"What version of the pipeline we are moving to"`
	Started      uint64                  `json:"started" example:"1712433802634" doc:"Time of deployment start in epoch milliseconds"`
	Ended        uint64                  `json:"ended" example:"1712433802634" doc:"Time of deployment end in epoch milliseconds"`
	State        DeploymentState         `json:"state" example:"RUNNING" doc:"The current state of the deployment as it exists within Gofer's operating model"`
	Status       DeploymentStatus        `json:"status" example:"SUCCESSFUL" doc:"The final status of the deployment"`
	StatusReason *DeploymentStatusReason `json:"status_reason" doc:"Details about a deployment's specific status"`
	Logs         []Event                 `json:"logs" doc:"The event logs from the deployment"`
}

func NewDeployment(namespace, pipeline string, id, startVersion, endVersion int64) *Deployment {
	return &Deployment{
		NamespaceID:  namespace,
		PipelineID:   pipeline,
		DeploymentID: id,
		StartVersion: startVersion,
		EndVersion:   endVersion,
		Started:      uint64(time.Now().UnixMilli()),
		Ended:        0,
		State:        DeploymentStateRunning,
		Status:       DeploymentStatusUnknown,
		StatusReason: nil,
		Logs:         []Event{},
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
		Namespace:    d.NamespaceID,
		Pipeline:     d.PipelineID,
		ID:           d.DeploymentID,
		StartVersion: d.StartVersion,
		EndVersion:   d.EndVersion,
		Started:      fmt.Sprint(d.Started),
		Ended:        fmt.Sprint(d.Ended),
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

	started, err := strconv.ParseUint(storage.Started, 10, 64)
	if err != nil {
		log.Fatal().Err(err).Msg("error in translating from storage")
	}

	ended, err := strconv.ParseUint(storage.Ended, 10, 64)
	if err != nil {
		log.Fatal().Err(err).Msg("error in translating from storage")
	}

	d.NamespaceID = storage.Namespace
	d.PipelineID = storage.Pipeline
	d.DeploymentID = storage.ID
	d.StartVersion = storage.StartVersion
	d.EndVersion = storage.EndVersion
	d.Started = started
	d.Ended = ended
	d.State = DeploymentState(storage.State)
	d.Status = DeploymentStatus(storage.Status)
	d.StatusReason = &statusReason
	d.Logs = logs
}
