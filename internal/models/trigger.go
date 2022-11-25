package models

import (
	"encoding/json"
	"time"

	"github.com/clintjedwards/gofer/internal/storage"
	proto "github.com/clintjedwards/gofer/proto/go"
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
	Registration ExtensionRegistration `json:"registration"`

	// URL is the network address used to communicate with the extension by the main process.
	URL           string         `json:"url"`
	Started       int64          `json:"started"` // The start time of the extension in epoch milliseconds.
	State         ExtensionState `json:"state"`
	Documentation string         `json:"documentation"` // The documentation link for this specific extension.
	// Key is a extension's authentication key used to validate requests from the Gofer main service.
	// On every request the Gofer service passes this key so that it is impossible for other service to contact
	// and manipulate extensions directly.
	Key *string `json:"-"`
}

func (t *Extension) ToProto() *proto.Extension {
	return &proto.Extension{
		Name:          t.Registration.Name,
		Image:         t.Registration.Image,
		Url:           t.URL,
		Started:       t.Started,
		State:         proto.Extension_ExtensionState(proto.Extension_ExtensionState_value[string(t.State)]),
		Status:        proto.Extension_ExtensionStatus(proto.Extension_ExtensionStatus_value[string(t.Registration.Status)]),
		Documentation: t.Documentation,
	}
}

// When installing a new extension, we allow the extension installer to pass a bunch of settings that
// allow us to go get that extension on future startups.
type ExtensionRegistration struct {
	Name         string          `json:"name"`
	Image        string          `json:"image"`
	RegistryAuth *RegistryAuth   `json:"registry_auth"`
	Variables    []Variable      `json:"variables"`
	Created      int64           `json:"created"`
	Status       ExtensionStatus `json:"status"`
}

func (c *ExtensionRegistration) FromInstallExtensionRequest(proto *proto.InstallExtensionRequest) {
	variables := []Variable{}
	for key, value := range proto.Variables {
		variables = append(variables, Variable{
			Key:    key,
			Value:  value,
			Source: VariableSourceSystem,
		})
	}

	var registryAuth *RegistryAuth
	if proto.User != "" {
		registryAuth = &RegistryAuth{
			User: proto.User,
			Pass: proto.Pass,
		}
	}

	c.Name = proto.Name
	c.Image = proto.Image
	c.RegistryAuth = registryAuth
	c.Variables = variables
	c.Created = time.Now().UnixMilli()
	c.Status = ExtensionStatusEnabled
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
		Name:         c.Name,
		Image:        c.Image,
		RegistryAuth: registryAuth,
		Variables:    variables,
		Created:      c.Created,
		Status:       string(c.Status),
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

	c.Name = tr.Name
	c.Image = tr.Image
	c.RegistryAuth = &registryAuth
	c.Variables = variables
	c.Created = tr.Created
	c.Status = ExtensionStatus(tr.Status)
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
	// The specific type of subscription failure. Good for documentation about what it might be.
	Reason      ExtensionSubscriptionStatusReasonKind `json:"kind"`
	Description string                                `json:"description"` // The description of why the run might have failed.
}

func (r *ExtensionSubscriptionStatusReason) ToJSON() string {
	reason, err := json.Marshal(r)
	if err != nil {
		log.Fatal().Err(err).Msg("failed to convert extension subscription status reason to json")
	}

	return string(reason)
}

func (r *ExtensionSubscriptionStatusReason) ToProto() *proto.PipelineExtensionSubscriptionStatusReason {
	return &proto.PipelineExtensionSubscriptionStatusReason{
		Reason: proto.PipelineExtensionSubscriptionStatusReason_PipelineExtensionSubscriptionStatusReasonKind(
			proto.PipelineExtensionSubscriptionStatusReason_PipelineExtensionSubscriptionStatusReasonKind_value[string(r.Reason)]),
		Description: r.Description,
	}
}

type PipelineExtensionSubscription struct {
	Namespace    string
	Pipeline     string
	Name         string
	Label        string
	Settings     map[string]string
	Status       ExtensionSubscriptionStatus
	StatusReason ExtensionSubscriptionStatusReason
}

func FromCreatePipelineExtensionSubscriptionRequest(request *proto.CreatePipelineExtensionSubscriptionRequest) *PipelineExtensionSubscription {
	return &PipelineExtensionSubscription{
		Namespace: request.NamespaceId,
		Pipeline:  request.PipelineId,
		Name:      request.ExtensionName,
		Label:     request.ExtensionLabel,
		Settings:  request.Settings,
		Status:    ExtensionSubscriptionStatusActive,
	}
}

func (ts *PipelineExtensionSubscription) ToStorage() *storage.PipelineExtensionSubscription {
	settings, err := json.Marshal(ts.Settings)
	if err != nil {
		log.Fatal().Err(err).Msg("error in translating from storage")
	}

	return &storage.PipelineExtensionSubscription{
		Namespace:    ts.Namespace,
		Pipeline:     ts.Pipeline,
		Name:         ts.Name,
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

	ts.Namespace = storage.Namespace
	ts.Pipeline = storage.Pipeline
	ts.Name = storage.Name
	ts.Label = storage.Label
	ts.Settings = settings
	ts.Status = ExtensionSubscriptionStatus(storage.Status)
	ts.StatusReason = statusReason
}

func (ts *PipelineExtensionSubscription) ToProto() *proto.PipelineExtensionSubscription {
	return &proto.PipelineExtensionSubscription{
		Namespace: ts.Namespace,
		Pipeline:  ts.Pipeline,
		Name:      ts.Name,
		Label:     ts.Label,
		Settings:  ts.Settings,
		Status: proto.PipelineExtensionSubscription_Status(
			proto.PipelineExtensionSubscriptionStatusReason_PipelineExtensionSubscriptionStatusReasonKind_value[string(ts.Status)]),
		StatusReason: ts.StatusReason.ToProto(),
	}
}
