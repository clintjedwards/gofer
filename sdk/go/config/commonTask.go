package config

import (
	"fmt"
	"strings"

	proto "github.com/clintjedwards/gofer/proto/go"
)

type CommonTaskWrapper struct {
	CommonTask
}

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

func (t *CommonTaskWrapper) Description(description string) *CommonTaskWrapper {
	t.CommonTask.Description = description
	return t
}

func (t *CommonTaskWrapper) DependsOnOne(taskID string, state RequiredParentStatus) *CommonTaskWrapper {
	t.CommonTask.DependsOn[taskID] = state
	return t
}

func (t *CommonTaskWrapper) DependsOnMany(dependsOn map[string]RequiredParentStatus) *CommonTaskWrapper {
	for id, status := range dependsOn {
		t.CommonTask.DependsOn[id] = status
	}
	return t
}

func (t *CommonTaskWrapper) Setting(key, value string) *CommonTaskWrapper {
	t.CommonTask.Settings[fmt.Sprintf("GOFER_PLUGIN_PARAM_%s", strings.ToUpper(key))] = value
	return t
}

func (t *CommonTaskWrapper) Settings(settings map[string]string) *CommonTaskWrapper {
	for key, value := range settings {
		t.CommonTask.Settings[fmt.Sprintf("GOFER_PLUGIN_PARAM_%s", strings.ToUpper(key))] = value
	}
	return t
}
