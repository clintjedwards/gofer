package sdk

import proto "github.com/clintjedwards/gofer/proto/go"

type CommonTaskConfig struct {
	Kind        TaskKind                        `json:"kind"`
	Name        string                          `json:"name"`
	Label       string                          `json:"label"`
	Description string                          `json:"description"`
	DependsOn   map[string]RequiredParentStatus `json:"depends_on"`
	Settings    map[string]string               `json:"settings"`
}

func (t *CommonTaskConfig) isTaskConfig() {}

func (t *CommonTaskConfig) getKind() TaskKind {
	return TaskKindCommon
}

func (t *CommonTaskConfig) getID() string {
	return t.Label
}

func (t *CommonTaskConfig) getDependsOn() map[string]RequiredParentStatus {
	return t.DependsOn
}

func NewCommonTask(name, label string) *CommonTaskConfig {
	return &CommonTaskConfig{
		Kind:        TaskKindCommon,
		Name:        name,
		Label:       label,
		Description: "",
		DependsOn:   make(map[string]RequiredParentStatus),
		Settings:    make(map[string]string),
	}
}

func (t *CommonTaskConfig) ToProto() *proto.CommonTaskConfig {
	dependsOn := map[string]proto.CommonTaskConfig_RequiredParentStatus{}
	for key, value := range t.DependsOn {
		dependsOn[key] = proto.CommonTaskConfig_RequiredParentStatus(proto.CommonTaskConfig_RequiredParentStatus_value[string(value)])
	}

	return &proto.CommonTaskConfig{
		Name:        t.Name,
		Label:       t.Label,
		Description: t.Description,
		DependsOn:   dependsOn,
		Settings:    t.Settings,
	}
}

func (t *CommonTaskConfig) validate() error {
	err := validateVariables(t.Settings)
	if err != nil {
		return err
	}
	return validateIdentifier("label", t.Label)
}

func (t *CommonTaskConfig) WithDescription(description string) *CommonTaskConfig {
	t.Description = description
	return t
}

func (t *CommonTaskConfig) WithDependsOnOne(taskID string, state RequiredParentStatus) *CommonTaskConfig {
	t.DependsOn[taskID] = state
	return t
}

func (t *CommonTaskConfig) WithDependsOnMany(dependsOn map[string]RequiredParentStatus) *CommonTaskConfig {
	for id, status := range dependsOn {
		t.DependsOn[id] = status
	}
	return t
}

func (t *CommonTaskConfig) WithSetting(key, value string) *CommonTaskConfig {
	t.Settings[key] = value
	return t
}

func (t *CommonTaskConfig) WithSettings(settings map[string]string) *CommonTaskConfig {
	for key, value := range settings {
		t.Settings[key] = value
	}
	return t
}
