package models

import (
	"time"

	proto "github.com/clintjedwards/gofer/proto/go"
)

type TriggerState string

const (
	TriggerStateUnknown    TriggerState = "UNKNOWN"    // Unknown state, can be in this state because of an error.
	TriggerStateProcessing TriggerState = "PROCESSING" // Pre-scheduling validation and prep.
	TriggerStateRunning    TriggerState = "RUNNING"    // Currently running as reported by scheduler.
	TriggerStateExited     TriggerState = "EXITED"     // Trigger has exited; usually because of an error.
)

type TriggerStatus string

const (
	TriggerStatusUnknown TriggerStatus = "UNKNOWN" // Cannot determine status of Trigger, should never be in this status.
	TriggerStatusEnabled TriggerStatus = "ENABLED" // Installed and able to be used by pipelines.
	/// Not available to be used by pipelines, either through lack of installation or
	/// being disabled by an admin.
	TriggerStatusDisabled TriggerStatus = "DISABLED"
)

type Trigger struct {
	Registration TriggerRegistration `json:"registration"`

	// URL is the network address used to communicate with the trigger by the main process.
	URL           string       `json:"url"`
	Started       int64        `json:"started"` // The start time of the trigger in epoch milliseconds.
	State         TriggerState `json:"state"`
	Documentation string       `json:"documentation"` // The documentation link for this specific trigger.
	// Key is a trigger's authentication key used to validate requests from the Gofer main service.
	// On every request the Gofer service passes this key so that it is impossible for other service to contact
	// and manipulate triggers directly.
	Key *string `json:"-"`
}

func (t *Trigger) ToProto() *proto.Trigger {
	return &proto.Trigger{
		Name:          t.Registration.Name,
		Image:         t.Registration.Image,
		Url:           t.URL,
		Started:       t.Started,
		State:         proto.Trigger_TriggerState(proto.Trigger_TriggerState_value[string(t.State)]),
		Status:        proto.Trigger_TriggerStatus(proto.Trigger_TriggerStatus_value[string(t.Registration.Status)]),
		Documentation: t.Documentation,
	}
}

// When installing a new trigger, we allow the trigger installer to pass a bunch of settings that
// allow us to go get that trigger on future startups.
type TriggerRegistration struct {
	Name         string        `json:"name"`
	Image        string        `json:"image"`
	RegistryAuth *RegistryAuth `json:"registry_auth"`
	Variables    []Variable    `json:"variables"`
	Created      int64         `json:"created"`
	Status       TriggerStatus `json:"status"`
}

func (c *TriggerRegistration) FromInstallTriggerRequest(proto *proto.InstallTriggerRequest) {
	variables := []Variable{}
	for key, value := range proto.Variables {
		variables = append(variables, Variable{
			Key:    key,
			Value:  value,
			Source: VariableSourceSystem,
		})
	}

	var registryAuth *RegistryAuth = nil
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
	c.Status = TriggerStatusEnabled
}
