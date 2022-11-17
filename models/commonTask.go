package models

import (
	"fmt"
	"strings"
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

// CommonTask is a representation of a Pipeline Common task. It combines both the settings for the common task
// stored exclusively with Gofer, but also a user pipeline's personal settings. This is so they can be combined and used
// in downstream task runs.
// To make this a little more clear, pipelines settings are stored on a per pipeline basis; while the registration
// configuration is stored just once and used globally.
type CommonTask struct {
	// Settings refers to settings passed to a common task by a specific pipeline.
	Settings PipelineCommonTaskSettings `json:"settings"`

	// Registration refers to the settings that the common task was installed with. These settings apply to all instances
	// run of the common task.
	Registration CommonTaskRegistration `json:"registration"`
}

func (t *CommonTask) isTask() {}

func (t *CommonTask) GetID() string {
	return t.Settings.Label
}

func (t *CommonTask) GetDescription() string {
	return t.Settings.Description
}

func (t *CommonTask) GetImage() string {
	return t.Registration.Image
}

func (t *CommonTask) GetRegistryAuth() *RegistryAuth {
	return t.Registration.RegistryAuth
}

func (t *CommonTask) GetDependsOn() map[string]RequiredParentStatus {
	return t.Settings.DependsOn
}

func (t *CommonTask) GetVariables() []Variable {
	variables := []Variable{}
	for key, value := range t.Settings.Settings {
		variables = append(variables, Variable{
			Key:    key,
			Value:  value,
			Source: VariableSourcePipelineConfig,
		})
	}

	variables = append(variables, t.Registration.Variables...)

	return variables
}

func (t *CommonTask) GetEntrypoint() *[]string {
	return nil
}

func (t *CommonTask) GetCommand() *[]string {
	return nil
}

func (t *CommonTask) GetInjectAPIToken() bool {
	return t.Settings.InjectAPIToken
}

func (t *CommonTask) ToProto() *proto.CommonTask {
	return &proto.CommonTask{
		Settings:     t.Settings.ToProto(),
		Registration: t.Registration.ToProto(),
	}
}

func (t *CommonTask) FromProto(pb *proto.CommonTask) {
	var registration CommonTaskRegistration
	registration.FromProto(pb.Registration)

	var settings PipelineCommonTaskSettings
	settings.FromProto(pb.Settings)

	t.Settings = settings
	t.Registration = registration
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

func (c *CommonTaskRegistration) ToProto() *proto.CommonTaskRegistration {
	variables := []*proto.Variable{}
	for _, v := range c.Variables {
		variables = append(variables, v.ToProto())
	}

	registryAuthUser := ""
	registryAuthPass := ""

	if c.RegistryAuth != nil {
		registryAuthUser = c.RegistryAuth.User
		registryAuthPass = c.RegistryAuth.Pass
	}

	return &proto.CommonTaskRegistration{
		Name:          c.Name,
		Image:         c.Image,
		User:          registryAuthUser,
		Pass:          registryAuthPass,
		Variables:     variables,
		Created:       c.Created,
		Status:        proto.CommonTaskRegistration_Status(proto.CommonTaskRegistration_Status_value[string(c.Status)]),
		Documentation: c.Documentation,
	}
}

func (c *CommonTaskRegistration) FromProto(proto *proto.CommonTaskRegistration) {
	variables := []Variable{}
	for _, variable := range proto.Variables {
		vari := Variable{}
		vari.FromProto(variable)
		variables = append(variables, vari)
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
	c.Status = CommonTaskStatusEnabled
	c.Documentation = proto.Documentation
}

func (c *CommonTaskRegistration) FromInstallCommonTaskRequest(proto *proto.InstallCommonTaskRequest) {
	variables := []Variable{}
	for key, value := range proto.Variables {
		variables = append(variables, Variable{
			Key:    fmt.Sprintf("GOFER_PLUGIN_CONFIG_%s", strings.ToUpper(key)),
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
	c.Status = CommonTaskStatusEnabled
	c.Documentation = proto.Documentation
}
