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

type Exec struct {
	Shell  string `json:"shell"`  // Which shell the user would like to use.
	Script string `json:"script"` // Base64 representation of the script.
}

func (r *Exec) ToProto() *proto.Exec {
	return &proto.Exec{
		Shell:  r.Shell,
		Script: r.Script,
	}
}

type Task struct {
	ID           string                         `json:"id"`
	Description  string                         `json:"description"`
	Image        string                         `json:"image"`
	RegistryAuth RegistryAuth                   `json:"registry_auth"`
	DependsOn    map[string]RequiredParentState `json:"depends_on"`
	EnvVars      map[string]string              `json:"env_vars"`
	// Secrets are passed in the exact same way as env_vars; but we don't allow the user to list the values.
	Secrets map[string]string `json:"secrets"`
	Exec    Exec              `json:"exec"` // Exec is a representation of a script to be run via container.
}

func (r *Task) ToProto() *proto.Task {
	dependsOn := map[string]proto.TaskRequiredParentState{}
	for key, value := range r.DependsOn {
		dependsOn[key] = proto.TaskRequiredParentState(proto.TaskRequiredParentState_value[string(value)])
	}

	// We want to show users what env_vars got passed into their task run without exposing certain secret values.
	// So we pass the proto a list of the keys that were passed in.
	secrets := []string{}
	for key := range r.Secrets {
		secrets = append(secrets, key)
	}

	return &proto.Task{
		Id:          r.ID,
		Description: r.Description,
		Image:       r.Image,
		DependsOn:   dependsOn,
		EnvVars:     r.EnvVars,
		Secrets:     secrets,
		Exec:        r.Exec.ToProto(),
	}
}
