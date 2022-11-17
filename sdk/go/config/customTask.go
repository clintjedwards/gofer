package config

import (
	proto "github.com/clintjedwards/gofer/proto/go"
)

// CustomTaskWrapper type simply exists so that we can make structs with fields like "id"
// and we can still add functions called "id()". This makes it not only easier to
// reason about when working with the struct, but when just writing pipelines as an end user.
type CustomTaskWrapper struct {
	CustomTask
}

// CustomTask is a representation of a Gofer custom task. Custom tasks are simply containers that
// Pipeline users need to run.
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
	// Allows users to tell gofer to auto-create and inject API Token into task. If this setting is found, Gofer creates
	// an API key for the run (stored in the user's secret store) and then injects it for this run under the
	// environment variables "GOFER_API_TOKEN". This key is automatically cleaned up when Gofer attempts to clean up
	// the Run's objects.
	InjectAPIToken bool `json:"inject_api_token"`
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

// Creates a new Gofer custom task. Custom Tasks are simple containers you wish to run.
func NewCustomTask(id, image string) *CustomTaskWrapper {
	return &CustomTaskWrapper{
		CustomTask{
			Kind:           TaskKindCustom,
			ID:             id,
			Description:    "",
			Image:          image,
			RegistryAuth:   nil,
			DependsOn:      make(map[string]RequiredParentStatus),
			Variables:      make(map[string]string),
			InjectAPIToken: false,
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
		Id:             t.CustomTask.ID,
		Description:    t.CustomTask.Description,
		Image:          t.CustomTask.Image,
		RegistryAuth:   t.CustomTask.RegistryAuth.Proto(),
		DependsOn:      dependsOn,
		Variables:      t.CustomTask.Variables,
		Entrypoint:     entrypoint,
		Command:        command,
		InjectApiToken: t.CustomTask.InjectAPIToken,
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
	t.CustomTask.InjectAPIToken = proto.InjectApiToken
}

func (t *CustomTaskWrapper) validate() error {
	err := validateVariables(t.CustomTask.Variables)
	if err != nil {
		return err
	}
	return validateIdentifier("id", t.ID)
}

// Add a short description of the task's purpose.
func (t *CustomTaskWrapper) Description(description string) *CustomTaskWrapper {
	t.CustomTask.Description = description
	return t
}

// Authentication details if your container repository requires them.
func (t *CustomTaskWrapper) RegistryAuth(user, pass string) *CustomTaskWrapper {
	t.CustomTask.RegistryAuth = &RegistryAuth{
		User: user,
		Pass: pass,
	}
	return t
}

// Add a single task dependency. This allows you to tie a task's execution to the result of another task.
func (t *CustomTaskWrapper) DependsOn(taskID string, state RequiredParentStatus) *CustomTaskWrapper {
	t.CustomTask.DependsOn[taskID] = state
	return t
}

// Add multiple task dependencies. This allows you to tie a task's execution to the result of several other tasks.
func (t *CustomTaskWrapper) DependsOnMany(dependsOn map[string]RequiredParentStatus) *CustomTaskWrapper {
	for id, status := range dependsOn {
		t.CustomTask.DependsOn[id] = status
	}
	return t
}

// Add a single variable. Variables are passed to your custom task as environment variables in a key value fashion.
// Variable values can also be pulled from other resources within Gofer. Making it easy to
// pass in things like secrets.
func (t *CustomTaskWrapper) Variable(key, value string) *CustomTaskWrapper {
	t.CustomTask.Variables[key] = value
	return t
}

// Add multiple variables. Variables are passed to your custom task as environment variables in a key value fashion.
// Variable values can also be pulled from other resources within Gofer like the secret store. Making it easy to
// pass in things like secrets.
func (t *CustomTaskWrapper) Variables(variables map[string]string) *CustomTaskWrapper {
	for key, value := range variables {
		t.CustomTask.Variables[key] = value
	}
	return t
}

// Change the container's [entrypoint](https://docs.docker.com/engine/reference/builder/#understand-how-cmd-and-entrypoint-interact
func (t *CustomTaskWrapper) Entrypoint(entrypoint ...string) *CustomTaskWrapper {
	t.CustomTask.Entrypoint = &entrypoint
	return t
}

// Change the container's [command](https://docs.docker.com/engine/reference/builder/#understand-how-cmd-and-entrypoint-interact)
func (t *CustomTaskWrapper) Command(command ...string) *CustomTaskWrapper {
	t.CustomTask.Command = &command
	return t
}

// Gofer will auto-generate and inject a Gofer API token as `GOFER_API_TOKEN`. This allows you to easily have tasks
// communicate with Gofer by either embedding Gofer's CLI or just simply using the token to authenticate to the API.
//
// This auto-generated token is stored in this pipeline's secret store and automatically cleaned up when the run
// objects get cleaned up.
func (t *CustomTaskWrapper) InjectAPIToken(injectToken bool) *CustomTaskWrapper {
	t.CustomTask.InjectAPIToken = injectToken
	return t
}
