package config

import (
	"fmt"
	"strings"

	proto "github.com/clintjedwards/gofer/proto/go"
)

// CommonTaskWrapper type simply exists so that we can make structs with fields like "id"
// and we can still add functions called "id()". This makes it not only easier to
// reason about when working with the struct, but when just writing pipelines as an end user.
type CommonTaskWrapper struct {
	CommonTask
}

// CommonTask represents pre-configured containers set by Gofer administrators that can be used
// as part of a pipeline.
type CommonTask struct {
	Kind        TaskKind                        `json:"kind"`
	Name        string                          `json:"name"`
	Label       string                          `json:"label"`
	Description string                          `json:"description"`
	DependsOn   map[string]RequiredParentStatus `json:"depends_on"`
	Settings    map[string]string               `json:"settings"`
}

func (t *CommonTaskWrapper) isTaskConfig() {}

func (t *CommonTaskWrapper) getKind() TaskKind {
	return TaskKindCommon
}

func (t *CommonTaskWrapper) getID() string {
	return t.CommonTask.Label
}

func (t *CommonTaskWrapper) getDependsOn() map[string]RequiredParentStatus {
	return t.CommonTask.DependsOn
}

// Creates a new Gofer common task. Common Tasks are Gofer administrator set containers that
// container pre-configured configuration such that you can use within your pipeline.
//
// For example, doing actions like posting to Slack would usually involve a common task rather than a custom one.
func NewCommonTask(name, label string) *CommonTaskWrapper {
	return &CommonTaskWrapper{
		CommonTask{
			Kind:        TaskKindCommon,
			Name:        name,
			Label:       label,
			Description: "",
			DependsOn:   make(map[string]RequiredParentStatus),
			Settings:    make(map[string]string),
		},
	}
}

func (t *CommonTaskWrapper) Proto() *proto.CommonTaskConfig {
	dependsOn := map[string]proto.CommonTaskConfig_RequiredParentStatus{}
	for key, value := range t.CommonTask.DependsOn {
		dependsOn[key] = proto.CommonTaskConfig_RequiredParentStatus(proto.CommonTaskConfig_RequiredParentStatus_value[string(value)])
	}

	return &proto.CommonTaskConfig{
		Name:        t.CommonTask.Name,
		Label:       t.CommonTask.Label,
		Description: t.CommonTask.Description,
		DependsOn:   dependsOn,
		Settings:    t.CommonTask.Settings,
	}
}

func (t *CommonTaskWrapper) validate() error {
	err := validateVariables(t.CommonTask.Settings)
	if err != nil {
		return err
	}
	return validateIdentifier("label", t.Label)
}

// Add a short description of the purpose of this common task.
func (t *CommonTaskWrapper) Description(description string) *CommonTaskWrapper {
	t.CommonTask.Description = description
	return t
}

// Add a single task dependency. This allows you to tie a task's execution to the result of another task.
func (t *CommonTaskWrapper) DependsOn(taskID string, state RequiredParentStatus) *CommonTaskWrapper {
	t.CommonTask.DependsOn[taskID] = state
	return t
}

// Add multiple task dependencies. This allows you to tie a task's execution to the result of several other tasks.
func (t *CommonTaskWrapper) DependsOnMany(dependsOn map[string]RequiredParentStatus) *CommonTaskWrapper {
	for id, status := range dependsOn {
		t.CommonTask.DependsOn[id] = status
	}
	return t
}

// Add a single setting. Settings allows you to control the behavior of common task's.
// Make sure to read the common task's readme in order to understand which settings and their
// associated values are accepted.
func (t *CommonTaskWrapper) Setting(key, value string) *CommonTaskWrapper {
	t.CommonTask.Settings[fmt.Sprintf("GOFER_PLUGIN_PARAM_%s", strings.ToUpper(key))] = value
	return t
}

// Add multiple settings. Settings allows you to control the behavior of common task's.
// Make sure to read the common task's readme in order to understand which settings and their
// associated values are accepted.
func (t *CommonTaskWrapper) Settings(settings map[string]string) *CommonTaskWrapper {
	for key, value := range settings {
		t.CommonTask.Settings[fmt.Sprintf("GOFER_PLUGIN_PARAM_%s", strings.ToUpper(key))] = value
	}
	return t
}
