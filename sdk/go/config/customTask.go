package config

import (
	proto "github.com/clintjedwards/gofer/proto/go"
)

type CustomTaskWrapper struct {
	CustomTask
}

type CustomTask struct {
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

func (t *CustomTaskWrapper) isTaskConfig() {}

func (t *CustomTaskWrapper) getKind() TaskKind {
	return TaskKindCustom
}

func (t *CustomTaskWrapper) getID() string {
	return t.CustomTask.ID
}

func (t *CustomTaskWrapper) getDependsOn() map[string]RequiredParentStatus {
	return t.CustomTask.DependsOn
}

func NewCustomTask(id, image string) *CustomTaskWrapper {
	return &CustomTaskWrapper{
		CustomTask{
			Kind:         TaskKindCustom,
			ID:           id,
			Description:  "",
			Image:        image,
			RegistryAuth: nil,
			DependsOn:    make(map[string]RequiredParentStatus),
			Variables:    make(map[string]string),
		},
	}
}

func (t *CustomTaskWrapper) Proto() *proto.CustomTaskConfig {
	dependsOn := map[string]proto.CustomTaskConfig_RequiredParentStatus{}
	for key, value := range t.CustomTask.DependsOn {
		dependsOn[key] = proto.CustomTaskConfig_RequiredParentStatus(proto.CustomTaskConfig_RequiredParentStatus_value[string(value)])
	}

	entrypoint := []string{}
	if t.CustomTask.Entrypoint != nil {
		entrypoint = *t.CustomTask.Entrypoint
	}

	command := []string{}
	if t.CustomTask.Command != nil {
		command = *t.CustomTask.Command
	}

	return &proto.CustomTaskConfig{
		Id:           t.CustomTask.ID,
		Description:  t.CustomTask.Description,
		Image:        t.CustomTask.Image,
		RegistryAuth: t.CustomTask.RegistryAuth.Proto(),
		DependsOn:    dependsOn,
		Variables:    t.CustomTask.Variables,
		Entrypoint:   entrypoint,
		Command:      command,
	}
}

func (t *CustomTaskWrapper) FromCustomTaskProto(proto *proto.CustomTaskConfig) {
	var registryAuth *RegistryAuth
	if proto.RegistryAuth != nil {
		ra := RegistryAuth{}
		ra.FromProto(proto.RegistryAuth)
		registryAuth = &ra
	}

	dependsOn := map[string]RequiredParentStatus{}
	for id, status := range proto.DependsOn {
		dependsOn[id] = RequiredParentStatus(status)
	}

	var entrypoint *[]string
	if len(proto.Entrypoint) != 0 {
		entrypoint = &proto.Entrypoint
	}

	var command *[]string
	if len(proto.Command) != 0 {
		command = &proto.Command
	}

	t.CustomTask.ID = proto.Id
	t.CustomTask.Description = proto.Description
	t.CustomTask.Image = proto.Image
	t.CustomTask.RegistryAuth = registryAuth
	t.CustomTask.DependsOn = dependsOn
	t.CustomTask.Variables = proto.Variables
	t.CustomTask.Entrypoint = entrypoint
	t.CustomTask.Command = command
}

func (t *CustomTaskWrapper) validate() error {
	err := validateVariables(t.CustomTask.Variables)
	if err != nil {
		return err
	}
	return validateIdentifier("id", t.ID)
}

func (t *CustomTaskWrapper) Description(description string) *CustomTaskWrapper {
	t.CustomTask.Description = description
	return t
}

func (t *CustomTaskWrapper) RegistryAuth(user, pass string) *CustomTaskWrapper {
	t.CustomTask.RegistryAuth = &RegistryAuth{
		User: user,
		Pass: pass,
	}
	return t
}

func (t *CustomTaskWrapper) DependsOn(taskID string, state RequiredParentStatus) *CustomTaskWrapper {
	t.CustomTask.DependsOn[taskID] = state
	return t
}

func (t *CustomTaskWrapper) DependsOnMany(dependsOn map[string]RequiredParentStatus) *CustomTaskWrapper {
	for id, status := range dependsOn {
		t.CustomTask.DependsOn[id] = status
	}
	return t
}

func (t *CustomTaskWrapper) Variable(key, value string) *CustomTaskWrapper {
	t.CustomTask.Variables[key] = value
	return t
}

func (t *CustomTaskWrapper) Variables(variables map[string]string) *CustomTaskWrapper {
	for key, value := range variables {
		t.CustomTask.Variables[key] = value
	}
	return t
}

func (t *CustomTaskWrapper) Entrypoint(entrypoint ...string) *CustomTaskWrapper {
	t.CustomTask.Entrypoint = &entrypoint
	return t
}

func (t *CustomTaskWrapper) Command(command ...string) *CustomTaskWrapper {
	t.CustomTask.Command = &command
	return t
}
