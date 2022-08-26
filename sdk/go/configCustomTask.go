package sdk

import (
	proto "github.com/clintjedwards/gofer/proto/go"
)

type CustomTaskConfig struct {
	Kind         TaskKind                        `json:"kind"`
	ID           string                          `json:"id"`
	Description  string                          `json:"description"`
	Image        string                          `json:"image"`
	RegistryAuth *RegistryAuth                   `json:"registry_auth"`
	DependsOn    map[string]RequiredParentStatus `json:"depends_on"`
	Variables    map[string]string               `json:"variables"`
	Entrypoint   *[]string                       `json:"entrypoint"`
	Command      *[]string                       `json:"command"`
}

func (t *CustomTaskConfig) isTaskConfig() {}

func (t *CustomTaskConfig) getKind() TaskKind {
	return TaskKindCustom
}

func (t *CustomTaskConfig) getID() string {
	return t.ID
}

func (t *CustomTaskConfig) getDependsOn() map[string]RequiredParentStatus {
	return t.DependsOn
}

func NewCustomTask(id, image string) *CustomTaskConfig {
	return &CustomTaskConfig{
		Kind:         TaskKindCustom,
		ID:           id,
		Description:  "",
		Image:        image,
		RegistryAuth: nil,
		DependsOn:    make(map[string]RequiredParentStatus),
		Variables:    make(map[string]string),
	}
}

func (t *CustomTaskConfig) FromCustomTaskProto(proto *proto.CustomTaskConfig) {
	var registryAuth *RegistryAuth = nil
	if proto.RegistryAuth != nil {
		ra := RegistryAuth{}
		ra.FromProto(proto.RegistryAuth)
		registryAuth = &ra
	}

	dependsOn := map[string]RequiredParentStatus{}
	for id, status := range proto.DependsOn {
		dependsOn[id] = RequiredParentStatus(status)
	}

	var entrypoint *[]string = nil
	if len(proto.Entrypoint) != 0 {
		entrypoint = &proto.Entrypoint
	}

	var command *[]string = nil
	if len(proto.Command) != 0 {
		command = &proto.Command
	}

	t.ID = proto.Id
	t.Description = proto.Description
	t.Image = proto.Image
	t.RegistryAuth = registryAuth
	t.DependsOn = dependsOn
	t.Variables = proto.Variables
	t.Entrypoint = entrypoint
	t.Command = command
}

func (t *CustomTaskConfig) validate() error {
	return validateIdentifier("id", t.ID)
}

func (t *CustomTaskConfig) WithDescription(description string) *CustomTaskConfig {
	t.Description = description
	return t
}

func (t *CustomTaskConfig) WithRegistryAuth(user, pass string) *CustomTaskConfig {
	t.RegistryAuth = &RegistryAuth{
		User: user,
		Pass: pass,
	}
	return t
}

func (t *CustomTaskConfig) WithDependsOnOne(taskID string, state RequiredParentStatus) *CustomTaskConfig {
	t.DependsOn[taskID] = state
	return t
}

func (t *CustomTaskConfig) WithDependsOnMany(dependsOn map[string]RequiredParentStatus) *CustomTaskConfig {
	for id, status := range dependsOn {
		t.DependsOn[id] = status
	}
	return t
}

func (t *CustomTaskConfig) WithVariable(key, value string) *CustomTaskConfig {
	t.Variables[key] = value
	return t
}

func (t *CustomTaskConfig) WithVariables(variables map[string]string) *CustomTaskConfig {
	for key, value := range variables {
		t.Variables[key] = value
	}
	return t
}

func (t *CustomTaskConfig) WithEntrypoint(entrypoint ...string) *CustomTaskConfig {
	t.Entrypoint = &entrypoint
	return t
}

func (t *CustomTaskConfig) WithCommand(command ...string) *CustomTaskConfig {
	t.Command = &command
	return t
}
