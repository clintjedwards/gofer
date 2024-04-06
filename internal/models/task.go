package models

import (
	"encoding/json"
	"strings"

	"github.com/clintjedwards/gofer/internal/storage"
	sdk "github.com/clintjedwards/gofer/sdk/go/config"
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
	ID             string                          `json:"id" example:"my_task_name" doc:"Unique identifier for the task"`
	Description    string                          `json:"description" example:"example description" doc:"A user provided description for what this task does"`
	Image          string                          `json:"image" example:"ubuntu:latest" doc:"Which container image to run for this specific task"`
	RegistryAuth   *RegistryAuth                   `json:"registry_auth" doc:"Auth credentials for the image's registry"`
	DependsOn      map[string]RequiredParentStatus `json:"depends_on" example:"{\"task_one\":\"SUCCESS\"}"`
	Variables      []Variable                      `json:"variables" doc:"Variables which will be passed in as env vars to the task"`
	Entrypoint     *[]string                       `json:"entrypoint,omitempty" example:"[\"printenv\"]" doc:"Command to run on init of container; can be overridden"`
	Command        *[]string                       `json:"command,omitempty" example:"[\"printenv\"]" doc:"Command to run on init of container; cannot be overridden"`
	InjectAPIToken bool                            `json:"inject_api_token" example:"true" doc:"Whether to inject a run specific Gofer API Key. Useful for using Gofer API within the container"`
}

func (r *Task) FromSDKUserPipelineTaskConfig(t *sdk.UserPipelineTaskConfig) {
	var regAuth *RegistryAuth = nil

	if t.RegistryAuth != nil {
		regAuth.User = t.RegistryAuth.User
		regAuth.Pass = t.RegistryAuth.Pass
	}

	dependsOn := map[string]RequiredParentStatus{}
	for key, value := range t.DependsOn {
		dependsOn[key] = RequiredParentStatus(value)
	}

	r.ID = t.ID
	r.Description = t.Description
	r.Image = t.Image
	r.RegistryAuth = regAuth
	r.DependsOn = dependsOn
	r.InjectAPIToken = t.InjectAPIToken

	var variables []Variable
	for key, value := range t.Variables {
		variables = append(variables, Variable{Key: key, Value: value, Source: VariableSourcePipelineConfig})
	}
	r.Variables = variables

	if len(t.Entrypoint) > 0 {
		r.Entrypoint = &t.Entrypoint
	} else {
		r.Entrypoint = nil
	}

	if len(t.Command) > 0 {
		r.Command = &t.Command
	} else {
		r.Command = nil
	}
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
