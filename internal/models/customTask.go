package models

import (
	"encoding/json"
	"strings"

	"github.com/clintjedwards/gofer/internal/storage"
	proto "github.com/clintjedwards/gofer/proto/go"
	"github.com/rs/zerolog/log"
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

func (r *CustomTask) FromProtoCustomTaskConfig(t *proto.UserCustomTaskConfig) {
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

func (r *CustomTask) FromStorage(t *storage.PipelineCustomTask) {
	var regAuth *RegistryAuth
	regAuth.FromStorage(t.RegistryAuth)

	var dependsOn map[string]RequiredParentStatus

	err := json.Unmarshal([]byte(t.DependsOn), &dependsOn)
	if err != nil {
		log.Fatal().Err(err).Msg("error in translating from storage")
	}

	var variables []Variable

	err = json.Unmarshal([]byte(t.Variables), &variables)
	if err != nil {
		log.Fatal().Err(err).Msg("error in translating from storage")
	}

	var entrypoint []string
	if t.Entrypoint != "" {
		err = json.Unmarshal([]byte(t.Entrypoint), &entrypoint)
		if err != nil {
			log.Fatal().Err(err).Msg("error in translating from storage")
		}
	}

	var command []string
	if t.Command != "" {
		err = json.Unmarshal([]byte(t.Command), &command)
		if err != nil {
			log.Fatal().Err(err).Msg("error in translating from storage")
		}
	}

	r.ID = t.ID
	r.Description = t.Description
	r.Image = t.Image
	r.RegistryAuth = regAuth
	r.DependsOn = dependsOn
	r.Variables = variables
	r.Entrypoint = &entrypoint
	r.Command = &command
	r.InjectAPIToken = t.InjectAPIToken
}

func (r *CustomTask) ToStorage(namespace, pipeline string, version int64) *storage.PipelineCustomTask {
	var regAuth string
	if r.RegistryAuth != nil {
		regAuth = r.RegistryAuth.ToStorage()
	}

	dependsOn, err := json.Marshal(r.DependsOn)
	if err != nil {
		log.Fatal().Err(err).Msg("error in translating to storage")
	}

	variables, err := json.Marshal(r.Variables)
	if err != nil {
		log.Fatal().Err(err).Msg("error in translating to storage")
	}

	var entrypoint []byte
	if r.Entrypoint != nil {
		entrypoint, err = json.Marshal(r.Entrypoint)
		if err != nil {
			log.Fatal().Err(err).Msg("error in translating to storage")
		}
	}

	var command []byte
	if r.Command != nil {
		command, err = json.Marshal(r.Command)
		if err != nil {
			log.Fatal().Err(err).Msg("error in translating from storage")
		}
	}

	return &storage.PipelineCustomTask{
		Namespace:             namespace,
		Pipeline:              pipeline,
		PipelineConfigVersion: version,
		ID:                    r.ID,
		Description:           r.Description,
		Image:                 r.Image,
		RegistryAuth:          regAuth,
		DependsOn:             string(dependsOn),
		Variables:             string(variables),
		Entrypoint:            string(entrypoint),
		Command:               string(command),
		InjectAPIToken:        r.InjectAPIToken,
	}
}
