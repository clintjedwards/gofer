package models

import "github.com/clintjedwards/gofer/proto"

type RequiredParentState string

const (
	RequiredParentStateUnknown RequiredParentState = "UNKNOWN"
	RequiredParentStateAny     RequiredParentState = "ANY"
	RequiredParentStateSuccess RequiredParentState = "SUCCESSFUL"
	RequiredParentStateFail    RequiredParentState = "FAILURE"
)

type Task struct {
	ID          string                         `json:"id"`
	Description string                         `json:"description"`
	ImageName   string                         `json:"image_name"`
	DependsOn   map[string]RequiredParentState `json:"depends_on"`
	EnvVars     map[string]string              `json:"env_vars"`
	Secrets     map[string]string              `json:"secrets"`
}

func (r *Task) ToProto() *proto.Task {
	dependsOn := map[string]proto.TaskRequiredParentState{}
	for key, value := range r.DependsOn {
		dependsOn[key] = proto.TaskRequiredParentState(proto.TaskRequiredParentState_value[string(value)])
	}

	return &proto.Task{
		Id:          r.ID,
		Description: r.Description,
		ImageName:   r.ImageName,
		DependsOn:   dependsOn,
		EnvVars:     r.EnvVars,
		Secrets:     r.Secrets,
	}
}
