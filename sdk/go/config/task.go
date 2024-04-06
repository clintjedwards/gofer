package config

// TaskWrapper type simply exists so that we can make structs with fields like "id"
// and we can still add functions called "id()". This makes it not only easier to
// reason about when working with the struct, but when just writing pipelines as an end user.
type TaskWrapper struct {
	Task
}

// Task is a representation of a Gofer task. Tasks are simply containers that Pipeline users need to run.
type Task struct {
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

// Creates a new Gofer task. Tasks are simply containers you wish to run.
func NewTask(id, image string) *TaskWrapper {
	return &TaskWrapper{
		Task{
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

func (t *TaskWrapper) ToUserPipelineTaskConfig() *UserPipelineTaskConfig {
	dependsOn := map[string]RequiredParentStatus{}
	for key, value := range t.Task.DependsOn {
		dependsOn[key] = value
	}

	entrypoint := []string{}
	if t.Task.Entrypoint != nil {
		entrypoint = *t.Task.Entrypoint
	}

	command := []string{}
	if t.Task.Command != nil {
		command = *t.Task.Command
	}

	return &UserPipelineTaskConfig{
		ID:             t.Task.ID,
		Description:    t.Task.Description,
		Image:          t.Task.Image,
		RegistryAuth:   t.Task.RegistryAuth,
		DependsOn:      t.Task.DependsOn,
		Variables:      t.Task.Variables,
		Entrypoint:     entrypoint,
		Command:        command,
		InjectAPIToken: t.Task.InjectAPIToken,
	}
}

func (t *TaskWrapper) validate() error {
	return validateIdentifier("id", t.Task.ID)
}

// Add a short description of the task's purpose.
func (t *TaskWrapper) Description(description string) *TaskWrapper {
	t.Task.Description = description
	return t
}

// Authentication details if your container repository requires them.
func (t *TaskWrapper) RegistryAuth(user, pass string) *TaskWrapper {
	t.Task.RegistryAuth = &RegistryAuth{
		User: user,
		Pass: pass,
	}
	return t
}

// Add a single task dependency. This allows you to tie a task's execution to the result of another task.
func (t *TaskWrapper) DependsOn(taskID string, state RequiredParentStatus) *TaskWrapper {
	t.Task.DependsOn[taskID] = state
	return t
}

// Add multiple task dependencies. This allows you to tie a task's execution to the result of several other tasks.
func (t *TaskWrapper) DependsOnMany(dependsOn map[string]RequiredParentStatus) *TaskWrapper {
	for id, status := range dependsOn {
		t.Task.DependsOn[id] = status
	}
	return t
}

// Add a single variable. Variables are passed to your task as environment variables in a key value fashion.
// Variable values can also be pulled from other resources within Gofer. Making it easy to
// pass in things like secrets.
func (t *TaskWrapper) Variable(key, value string) *TaskWrapper {
	t.Task.Variables[key] = value
	return t
}

// Add multiple variables. Variables are passed to your task as environment variables in a key value fashion.
// Variable values can also be pulled from other resources within Gofer like the secret store. Making it easy to
// pass in things like secrets.
func (t *TaskWrapper) Variables(variables map[string]string) *TaskWrapper {
	for key, value := range variables {
		t.Task.Variables[key] = value
	}
	return t
}

// Change the container's [entrypoint](https://docs.docker.com/engine/reference/builder/#understand-how-cmd-and-entrypoint-interact
func (t *TaskWrapper) Entrypoint(entrypoint ...string) *TaskWrapper {
	t.Task.Entrypoint = &entrypoint
	return t
}

// Change the container's [command](https://docs.docker.com/engine/reference/builder/#understand-how-cmd-and-entrypoint-interact)
func (t *TaskWrapper) Command(command ...string) *TaskWrapper {
	t.Task.Command = &command
	return t
}

// Gofer will auto-generate and inject a Gofer API token as `GOFER_API_TOKEN`. This allows you to easily have tasks
// communicate with Gofer by either embedding Gofer's CLI or just simply using the token to authenticate to the API.
//
// This auto-generated token is stored in this pipeline's secret store and automatically cleaned up when the run
// objects get cleaned up.
func (t *TaskWrapper) InjectAPIToken(injectToken bool) *TaskWrapper {
	t.Task.InjectAPIToken = injectToken
	return t
}
