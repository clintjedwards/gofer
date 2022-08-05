package models

import (
	"time"

	proto "github.com/clintjedwards/gofer/proto/go"
)

type CommonTaskStatus string

const (
	CommonTaskStatusUnknown CommonTaskStatus = "UNKNOWN" // Cannot determine status of CommonTask, should never be in this status.
	CommonTaskStatusEnabled CommonTaskStatus = "ENABLED" // Installed and able to be used by pipelines.
	/// Not available to be used by pipelines, either through lack of installation or
	/// being disabled by an admin.
	CommonTaskStatusDisabled CommonTaskStatus = "DISABLED"
)

type CommonTask struct {
	Name          string           `json:"name"`
	Image         string           `json:"image"`
	RegistryAuth  *RegistryAuth    `json:"registry_auth"`
	Variables     []Variable       `json:"variables"`
	Status        CommonTaskStatus `json:"status"`
	Documentation *string          `json:"documentation"`
}

func (ct *CommonTask) ToProto() *proto.CommonTask {
	var docs string = ""
	if ct.Documentation != nil {
		docs = *ct.Documentation
	}

	return &proto.CommonTask{
		Name:          ct.Name,
		Image:         ct.Image,
		Documentation: docs,
		Status:        proto.CommonTask_Status(proto.CommonTask_Status_value[string(ct.Status)]),
	}
}

// When installing a new common task, we allow the common task installer to pass a bunch of settings that
// allow us to go get that common task on future startups.
type CommonTaskRegistration struct {
	Name          string           `json:"name"`
	Image         string           `json:"image"`
	RegistryAuth  *RegistryAuth    `json:"registry_auth"`
	Variables     []Variable       `json:"variables"`
	Created       int64            `json:"created"`
	Status        CommonTaskStatus `json:"status"`
	Documentation string           `json:"documentation"`
}

func (c *CommonTaskRegistration) FromInstallCommonTaskRequest(proto *proto.InstallCommonTaskRequest) {
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
	c.Status = CommonTaskStatusEnabled
	c.Documentation = proto.Documentation
}
