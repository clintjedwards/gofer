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

// CommonTask is a representation of a Pipeline Common task. It combines not only the settings for the common task
// store exclusivly with Gofer, but also the pipeline's personal settings. This is so they can be combined and used
// in downstream task runs.
type CommonTask struct {
	Settings     PipelineCommonTaskSettings `json:"settings"`
	Registration CommonTaskRegistration     `json:"registration"`
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

func (t *CommonTask) ToProto() *proto.CommonTask {
	dependsOn := map[string]proto.CommonTask_RequiredParentStatus{}
	for key, value := range t.GetDependsOn() {
		dependsOn[key] = proto.CommonTask_RequiredParentStatus(proto.CommonTask_RequiredParentStatus_value[string(value)])
	}

	variables := []*proto.Variable{}
	for _, v := range t.GetVariables() {
		variables = append(variables, v.ToProto())
	}

	return &proto.CommonTask{
		Id:           t.GetID(),
		Description:  t.GetDescription(),
		Image:        t.GetImage(),
		RegistryAuth: t.GetRegistryAuth().ToProto(),
		DependsOn:    dependsOn,
		Variables:    variables,
		Label:        t.Settings.Label,
		Name:         t.Settings.Name,
	}
}

func (t *CommonTask) FromProto(pb *proto.CommonTask) {
	dependsOn := map[string]RequiredParentStatus{}
	for id, status := range pb.DependsOn {
		dependsOn[id] = RequiredParentStatus(status.String())
	}

	variablesMap := map[string]string{}
	for _, v := range pb.Variables {
		variablesMap[v.Key] = v.Value
	}

	variablesList := []Variable{}
	for _, v := range pb.Variables {
		variable := Variable{}
		variable.FromProto(v)
		variablesList = append(variablesList, variable)
	}

	var regAuth *RegistryAuth
	regAuth.FromProto(pb.RegistryAuth)

	t.Settings = PipelineCommonTaskSettings{
		Name:        pb.Name,
		Label:       pb.Label,
		Description: pb.Description,
		DependsOn:   dependsOn,
		Settings:    variablesMap,
	}

	t.Registration = CommonTaskRegistration{
		Name:          pb.Name,
		Image:         pb.Image,
		RegistryAuth:  regAuth,
		Variables:     variablesList,
		Created:       0,
		Status:        CommonTaskStatusUnknown,
		Documentation: "",
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

func (c *CommonTaskRegistration) FromInstallCommonTaskRequest(proto *proto.InstallCommonTaskRequest) {
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
	c.Status = CommonTaskStatusEnabled
	c.Documentation = proto.Documentation
}
