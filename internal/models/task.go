package models

import "github.com/clintjedwards/gofer/proto"

type RequiredParentState string

const (
	RequiredParentStateUnknown RequiredParentState = "UNKNOWN"
	RequiredParentStateAny     RequiredParentState = "ANY"
	RequiredParentStateSuccess RequiredParentState = "SUCCESSFUL"
	RequiredParentStateFail    RequiredParentState = "FAILURE"
)

type RegistryAuth struct {
	User string `json:"user"`
	Pass string `json:"pass"`
}

func (r *RegistryAuth) ToProto() *proto.RegistryAuth {
	return &proto.RegistryAuth{
		User: r.User,
		Pass: r.Pass,
	}
}

type Task struct {
	ID           string                         `json:"id"`
	Description  string                         `json:"description"`
	Image        string                         `json:"image"`
	RegistryAuth RegistryAuth                   `json:"registry_auth"`
	DependsOn    map[string]RequiredParentState `json:"depends_on"`
	EnvVars      map[string]string              `json:"env_vars"`
}

func (r *Task) ToProto() *proto.Task {
	dependsOn := map[string]proto.TaskRequiredParentState{}
	for key, value := range r.DependsOn {
		dependsOn[key] = proto.TaskRequiredParentState(proto.TaskRequiredParentState_value[string(value)])
	}

	return &proto.Task{
		Id:          r.ID,
		Description: r.Description,
		Image:       r.Image,
		DependsOn:   dependsOn,
		EnvVars:     r.EnvVars,
	}
}
