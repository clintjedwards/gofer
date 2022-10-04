package config

import (
	"fmt"
	"strings"

	proto "github.com/clintjedwards/gofer/proto/go"
)

type PipelineTriggerConfig struct {
	Name     string            `json:"name"`
	Label    string            `json:"label"`
	Settings map[string]string `json:"settings"`
}

func NewTrigger(name, label string) *PipelineTriggerConfig {
	return &PipelineTriggerConfig{
		Name:  name,
		Label: label,
	}
}

func (p *PipelineTriggerConfig) WithSetting(key, value string) *PipelineTriggerConfig {
	p.Settings[fmt.Sprintf("GOFER_PLUGIN_PARAM_%s", strings.ToUpper(key))] = value
	return p
}

func (p *PipelineTriggerConfig) WithSettings(settings map[string]string) *PipelineTriggerConfig {
	for key, value := range settings {
		p.Settings[fmt.Sprintf("GOFER_PLUGIN_PARAM_%s", strings.ToUpper(key))] = value
	}
	return p
}

func (p *PipelineTriggerConfig) FromProto(proto *proto.PipelineTriggerConfig) {
	p.Name = proto.Name
	p.Label = proto.Label
	p.Settings = proto.Settings
}

func (p *PipelineTriggerConfig) ToProto() *proto.PipelineTriggerConfig {
	return &proto.PipelineTriggerConfig{
		Name:     p.Name,
		Label:    p.Label,
		Settings: p.Settings,
	}
}

func (p *PipelineTriggerConfig) validate() error {
	return validateIdentifier("label", p.Label)
}
