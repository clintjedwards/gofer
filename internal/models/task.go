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

type Task struct {
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

func (r *Task) ToProto() *proto.Task {
	dependsOn := map[string]proto.Task_RequiredParentStatus{}
	for key, value := range r.DependsOn {
		dependsOn[key] = proto.Task_RequiredParentStatus(proto.Task_RequiredParentStatus_value[string(value)])
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

	return &proto.Task{
		Id:             r.ID,
		Description:    r.Description,
		Image:          r.Image,
		RegistryAuth:   r.RegistryAuth.ToProto(),
		DependsOn:      dependsOn,
		Variables:      variables,
		Entrypoint:     entrypoint,
		Command:        command,
		InjectApiToken: r.InjectAPIToken,
	}
}

func (r *Task) FromProtoPipelineTaskConfig(t *proto.UserPipelineTaskConfig) {
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

func (r *Task) FromStorage(t *storage.PipelineTask) {
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

func (r *Task) ToStorage(namespace, pipeline string, version int64) *storage.PipelineTask {
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

	return &storage.PipelineTask{
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
