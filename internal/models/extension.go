package models

import (
	"encoding/json"
	"fmt"
	"strconv"

	"github.com/clintjedwards/gofer/internal/storage"
	"github.com/rs/zerolog/log"
)

type ExtensionState string

const (
	ExtensionStateUnknown    ExtensionState = "UNKNOWN"    // Unknown state, can be in this state because of an error.
	ExtensionStateProcessing ExtensionState = "PROCESSING" // Pre-scheduling validation and prep.
	ExtensionStateRunning    ExtensionState = "RUNNING"    // Currently running as reported by scheduler.
	ExtensionStateExited     ExtensionState = "EXITED"     // Extension has exited; usually because of an error.
)

type ExtensionStatus string

const (
	ExtensionStatusUnknown ExtensionStatus = "UNKNOWN" // Cannot determine status of Extension, should never be in this status.
	ExtensionStatusEnabled ExtensionStatus = "ENABLED" // Installed and able to be used by pipelines.
	/// Not available to be used by pipelines, either through lack of installation or
	/// being disabled by an admin.
	ExtensionStatusDisabled ExtensionStatus = "DISABLED"
)

type Extension struct {
	Registration ExtensionRegistration `json:"registration" doc:"Metadata about the extension as it's registered in Gofer"`

	URL           string         `json:"url" doc:"URL is the network address used to communicate with the extension by the main process"`
	Started       uint64         `json:"started" example:"1712433802634" doc:"The start time of the extension in epoch milliseconds"`
	State         ExtensionState `json:"state" example:"RUNNING" doc:"The current state of the extension as it exists within Gofer's operating model."`
	Documentation string         `json:"documentation" doc:"extension given documentation; supports markdown"`
	// Key is a extension's authentication key used to validate requests from the Gofer main service.
	// On every request the Gofer service passes this key so that it is impossible for other service to contact
	// and manipulate extensions directly.
	Key *string `json:"-" hidden:"true"`
}

// When installing a new extension, we allow the extension installer to pass a bunch of settings that
// allow us to go get that extension on future startups.
type ExtensionRegistration struct {
	ID           string          `json:"id" example:"cron" doc:"Unique identifier for the extension"`
	Image        string          `json:"image" example:"ubuntu:latest" doc:"Which container image this extension should run"`
	RegistryAuth *RegistryAuth   `json:"registry_auth" doc:"Auth credentials for the image's registry"`
	Variables    []Variable      `json:"variables" doc:"Variables which will be passed in as env vars to the task"`
	Created      uint64          `json:"created" example:"1712433802634" doc:"Time of pipeline creation in epoch milliseconds"`
	Status       ExtensionStatus `json:"status" example:"ENABLED" doc:"Whether the extension is enabled or not; extensions can be disabled to prevent use by admins"`
	KeyID        string          `json:"-" hidden:"true"`
}

func (c *ExtensionRegistration) ToStorage() *storage.GlobalExtensionRegistration {
	var registryAuth string

	if c.RegistryAuth != nil {
		registryAuth = c.RegistryAuth.ToStorage()
	}

	var variables string

	output, err := json.Marshal(c.Variables)
	if err != nil {
		log.Fatal().Err(err).Msg("could not (un)marshal from storage")
	}

	variables = string(output)

	storage := &storage.GlobalExtensionRegistration{
		ID:           c.ID,
		Image:        c.Image,
		RegistryAuth: registryAuth,
		Variables:    variables,
		Created:      fmt.Sprint(c.Created),
		Status:       string(c.Status),
		KeyID:        c.KeyID,
	}
	return storage
}

func (c *ExtensionRegistration) FromStorage(tr *storage.GlobalExtensionRegistration) {
	var registryAuth RegistryAuth

	if tr.RegistryAuth != "" {
		err := json.Unmarshal([]byte(tr.RegistryAuth), &registryAuth)
		if err != nil {
			log.Fatal().Err(err).Msg("could not (un)marshal from storage")
		}
	}

	var variables []Variable

	if tr.Variables != "" {
		err := json.Unmarshal([]byte(tr.Variables), &variables)
		if err != nil {
			log.Fatal().Err(err).Msg("could not (un)marshal from storage")
		}
	}

	created, err := strconv.ParseUint(tr.Created, 10, 64)
	if err != nil {
		log.Fatal().Err(err).Msg("error in translating from storage")
	}

	c.ID = tr.ID
	c.Image = tr.Image
	c.RegistryAuth = &registryAuth
	c.Variables = variables
	c.Created = created
	c.Status = ExtensionStatus(tr.Status)
	c.KeyID = tr.KeyID
}

type ExtensionSubscriptionStatus string

const (
	ExtensionSubscriptionStatusUnknown ExtensionSubscriptionStatus = "UNKNOWN"
	// Subscription is successfully connected and active.
	ExtensionSubscriptionStatusActive ExtensionSubscriptionStatus = "ACTIVE"
	// Subscription is not connected and inactive due to error.
	ExtensionSubscriptionStatusError ExtensionSubscriptionStatus = "ERROR"
	// Subscription is connected, but inactive due to user or operator request.
	ExtensionSubscriptionStatusDisabled ExtensionSubscriptionStatus = "DISABLED"
)

type ExtensionSubscriptionStatusReasonKind string

const (
	ExtensionSubscriptionStatusReasonUnknown                     ExtensionSubscriptionStatusReasonKind = "UNKNOWN"
	ExtensionSubscriptionStatusReasonExtensionNotFound           ExtensionSubscriptionStatusReasonKind = "EXTENSION_NOT_FOUND"
	ExtensionSubscriptionStatusReasonExtensionSubscriptionFailed ExtensionSubscriptionStatusReasonKind = "EXTENSION_SUBSCRIPTION_FAILED"
)

type ExtensionSubscriptionStatusReason struct {
	Reason      ExtensionSubscriptionStatusReasonKind `json:"reason" example:"EXTENSION_NOT_FOUND" doc:"Specific type of subscription failure"`
	Description string                                `json:"description" doc:"The description of why the run might have failed"`
}

func (r *ExtensionSubscriptionStatusReason) ToJSON() string {
	reason, err := json.Marshal(r)
	if err != nil {
		log.Fatal().Err(err).Msg("failed to convert extension subscription status reason to json")
	}

	return string(reason)
}

type PipelineExtensionSubscription struct {
	NamespaceID  string                            `json:"namespace_id" example:"default" doc:"Unique identifier of the target namespace"`
	PipelineID   string                            `json:"pipeline_id" example:"simple_pipeline" doc:"Unique identifier for the target pipeline"`
	ExtensionID  string                            `json:"name" example:"extension_id" doc:"Unique identifier for the target extension"`
	Label        string                            `json:"label" example:"every_5_seconds" doc:"A per pipeline unique identifier to differentiate multiple subscriptions to a single pipeline"`
	Settings     map[string]string                 `json:"settings" doc:"Each extension defines per pipeline settings that the user can subscribe with to perform different functionalities; These are generally listed in the extension documentation and passed through here."`
	Status       ExtensionSubscriptionStatus       `json:"status" example:"ACTIVE" doc:"The state of the subscription for the pipeline; defines whether this subscription is still active."`
	StatusReason ExtensionSubscriptionStatusReason `json:"status_reason" doc:"More details about why a subscription has a particular status"`
}

// func FromCreatePipelineExtensionSubscriptionRequest(request *CreatePipelineExtensionSubscriptionRequest) *PipelineExtensionSubscription {
// 	return &PipelineExtensionSubscription{
// 		NamespaceID: request.NamespaceId,
// 		PipelineID:  request.PipelineId,
// 		ExtensionID: request.ExtensionName,
// 		Label:       request.ExtensionLabel,
// 		Settings:    request.Settings,
// 		Status:      ExtensionSubscriptionStatusActive,
// 	}
// }

func (ts *PipelineExtensionSubscription) ToStorage() *storage.PipelineExtensionSubscription {
	settings, err := json.Marshal(ts.Settings)
	if err != nil {
		log.Fatal().Err(err).Msg("error in translating from storage")
	}

	return &storage.PipelineExtensionSubscription{
		Namespace:    ts.NamespaceID,
		Pipeline:     ts.PipelineID,
		ID:           ts.ExtensionID,
		Label:        ts.Label,
		Settings:     string(settings),
		Status:       string(ts.Status),
		StatusReason: ts.StatusReason.ToJSON(),
	}
}

func (ts *PipelineExtensionSubscription) FromStorage(storage *storage.PipelineExtensionSubscription) {
	var statusReason ExtensionSubscriptionStatusReason
	err := json.Unmarshal([]byte(storage.StatusReason), &statusReason)
	if err != nil {
		log.Fatal().Err(err).Msg("error in translating from storage")
	}

	var settings map[string]string
	err = json.Unmarshal([]byte(storage.Settings), &settings)
	if err != nil {
		log.Fatal().Err(err).Msg("error in translating from storage")
	}

	ts.NamespaceID = storage.Namespace
	ts.PipelineID = storage.Pipeline
	ts.ExtensionID = storage.ID
	ts.Label = storage.Label
	ts.Settings = settings
	ts.Status = ExtensionSubscriptionStatus(storage.Status)
	ts.StatusReason = statusReason
}
