package models

import (
	"strings"

	proto "github.com/clintjedwards/gofer/proto/go"

	sdk "github.com/clintjedwards/gofer/sdk/go"
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
	Entrypoint   []string                        `json:"entrypoint"`
	Command      []string                        `json:"command"`
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

	return &proto.Task{
		Id:          r.ID,
		Description: r.Description,
		Image:       r.Image,
		DependsOn:   dependsOn,
		Variables:   variables,
		Entrypoint:  r.Entrypoint,
		Command:     r.Command,
	}
}

func (r *Task) FromProto(t *proto.Task) {
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

	r.ID = t.Id
	r.Description = t.Description
	r.Image = t.Image
	r.DependsOn = dependsOn
	r.Variables = variables
	r.Entrypoint = t.Entrypoint
	r.Command = t.Command
}

func FromTaskConfig(t *sdk.Task) Task {
	dependsOn := map[string]RequiredParentStatus{}
	for key, value := range t.DependsOn {
		dependsOn[key] = RequiredParentStatus(value.ToString())
	}

	variables := []Variable{}
	for key, value := range t.Variables {
		variables = append(variables, Variable{
			Key:    key,
			Value:  value,
			Source: "Task configuration",
		})
	}

	return Task{
		ID:           t.ID,
		Description:  t.Description,
		Image:        t.Image,
		RegistryAuth: (*RegistryAuth)(t.RegistryAuth),
		DependsOn:    dependsOn,
		Variables:    variables,
		Entrypoint:   t.Entrypoint,
		Command:      t.Command,
	}
}
