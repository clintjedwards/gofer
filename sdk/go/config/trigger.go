package config

import (
	"fmt"
	"strings"

	proto "github.com/clintjedwards/gofer/proto/go"
)

type TriggerWrapper struct {
	Trigger
}

type Trigger struct {
	Name     string            `json:"name"`
	Label    string            `json:"label"`
	Settings map[string]string `json:"settings"`
}

func NewTrigger(name, label string) *TriggerWrapper {
	return &TriggerWrapper{
		Trigger{
			Name:  name,
			Label: label,
		},
	}
}

func (p *TriggerWrapper) Setting(key, value string) *TriggerWrapper {
	p.Trigger.Settings[fmt.Sprintf("GOFER_PLUGIN_PARAM_%s", strings.ToUpper(key))] = value
	return p
}

func (p *TriggerWrapper) Settings(settings map[string]string) *TriggerWrapper {
	for key, value := range settings {
		p.Trigger.Settings[fmt.Sprintf("GOFER_PLUGIN_PARAM_%s", strings.ToUpper(key))] = value
	}
	return p
}

func (p *TriggerWrapper) FromProto(proto *proto.PipelineTriggerConfig) {
	p.Trigger.Name = proto.Name
	p.Trigger.Label = proto.Label
	p.Trigger.Settings = proto.Settings
}

func (p *TriggerWrapper) Proto() *proto.PipelineTriggerConfig {
	return &proto.PipelineTriggerConfig{
		Name:     p.Trigger.Name,
		Label:    p.Trigger.Label,
		Settings: p.Trigger.Settings,
	}
}

func (p *TriggerWrapper) validate() error {
	return validateIdentifier("label", p.Label)
}
