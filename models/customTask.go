package models

import (
	"strings"

	proto "github.com/clintjedwards/gofer/proto/go"
)

type RequiredParentStatus string

const (
	RequiredParentStatusUnknown RequiredParentStatus = "UNKNOWN"
	RequiredParentStatusAny     RequiredParentStatus = "ANY"
	RequiredParentStatusSuccess RequiredParentStatus = "SUCCESS"
	RequiredParentStatusFailure RequiredParentStatus = "FAILURE"
)

func (s *RequiredParentStatus) FromStr(input string) RequiredParentStatus {
	switch strings.ToLower(input) {
	case "unknown":
		return RequiredParentStatusUnknown
	case "any":
		return RequiredParentStatusAny
	case "success":
		return RequiredParentStatusSuccess
	case "failure":
		return RequiredParentStatusFailure
	default:
		return RequiredParentStatusUnknown
	}
}

type CustomTask struct {
	ID           string                          `json:"id"`
	Description  string                          `json:"description"`
	Image        string                          `json:"image"`
	RegistryAuth *RegistryAuth                   `json:"registry_auth"`
	DependsOn    map[string]RequiredParentStatus `json:"depends_on"`
	Variables    []Variable                      `json:"variables"`
	Entrypoint   *[]string                       `json:"entrypoint"`
	Command      *[]string                       `json:"command"`
	// Allows users to tell gofer to auto-create and inject API Token into task. If this setting is found, Gofer creates
	// an API key for the run (stored in the user's secret store) and then injects it for this run under the
	// environment variables "GOFER_API_TOKEN". This key is automatically cleaned up when Gofer attempts to clean up
	// the Run's objects.
	InjectAPIToken bool `json:"inject_api_token"`
}

func (r *CustomTask) isTask() {}

func (r *CustomTask) GetID() string {
	return r.ID
}

func (r *CustomTask) GetDescription() string {
	return r.Description
}

func (r *CustomTask) GetImage() string {
	return r.Image
}

func (r *CustomTask) GetRegistryAuth() *RegistryAuth {
	return r.RegistryAuth
}

func (r *CustomTask) GetDependsOn() map[string]RequiredParentStatus {
	return r.DependsOn
}

func (r *CustomTask) GetVariables() []Variable {
	return r.Variables
}

func (r *CustomTask) GetEntrypoint() *[]string {
	return r.Entrypoint
}

func (r *CustomTask) GetCommand() *[]string {
	return r.Command
}

func (r *CustomTask) GetInjectAPIToken() bool {
	return r.InjectAPIToken
}

func (r *CustomTask) ToProto() *proto.CustomTask {
	dependsOn := map[string]proto.CustomTask_RequiredParentStatus{}
	for key, value := range r.DependsOn {
		dependsOn[key] = proto.CustomTask_RequiredParentStatus(proto.CustomTask_RequiredParentStatus_value[string(value)])
	}

	variables := []*proto.Variable{}
	for _, v := range r.Variables {
		variables = append(variables, v.ToProto())
	}

	entrypoint := []string{}
	if r.Entrypoint != nil {
		entrypoint = *r.Entrypoint
	}

	command := []string{}
	if r.Command != nil {
		command = *r.Command
	}

	return &proto.CustomTask{
		Id:             r.ID,
		Description:    r.Description,
		Image:          r.Image,
		RegistryAuth:   r.GetRegistryAuth().ToProto(),
		DependsOn:      dependsOn,
		Variables:      variables,
		Entrypoint:     entrypoint,
		Command:        command,
		InjectApiToken: r.InjectAPIToken,
	}
}

func (r *CustomTask) FromProto(t *proto.CustomTask) {
	dependsOn := map[string]RequiredParentStatus{}
	for id, status := range t.DependsOn {
		dependsOn[id] = RequiredParentStatus(status.String())
	}

	variables := []Variable{}
	for _, v := range t.Variables {
		variable := Variable{}
		variable.FromProto(v)
		variables = append(variables, variable)
	}

	var entrypoint *[]string
	if len(t.Entrypoint) != 0 {
		entrypoint = &t.Entrypoint
	}

	var command *[]string
	if len(t.Command) != 0 {
		command = &t.Command
	}

	r.ID = t.Id
	r.Description = t.Description
	r.Image = t.Image
	r.DependsOn = dependsOn
	r.Variables = variables
	r.Entrypoint = entrypoint
	r.Command = command
	r.InjectAPIToken = t.InjectApiToken
}

func (r *CustomTask) FromProtoCustomTaskConfig(t *proto.CustomTaskConfig) {
	dependsOn := map[string]RequiredParentStatus{}
	for key, value := range t.DependsOn {
		dependsOn[key] = RequiredParentStatus(value.String())
	}

	variables := []Variable{}
	for key, value := range t.Variables {
		variables = append(variables, Variable{
			Key:    key,
			Value:  value,
			Source: VariableSourcePipelineConfig,
		})
	}

	var regAuth *RegistryAuth
	regAuth.FromProto(t.RegistryAuth)

	var entrypoint *[]string
	if len(t.Entrypoint) > 0 {
		entrypoint = &t.Entrypoint
	}

	var command *[]string
	if len(t.Command) > 0 {
		command = &t.Command
	}

	r.ID = t.Id
	r.Description = t.Description
	r.Image = t.Image
	r.DependsOn = dependsOn
	r.Variables = variables
	r.Entrypoint = entrypoint
	r.Command = command
	r.InjectAPIToken = t.InjectApiToken
}
