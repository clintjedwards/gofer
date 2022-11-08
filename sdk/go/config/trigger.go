package config

import (
	"fmt"
	"strings"

	proto "github.com/clintjedwards/gofer/proto/go"
)

// TriggerWrapper type simply exists so that we can make structs with fields like "id"
// and we can still add functions called "id()". This makes it not only easier to
// reason about when working with the struct, but when just writing pipelines as an end user.
type TriggerWrapper struct {
	Trigger
}

// Trigger is a representation of a Gofer Trigger. Triggers are Gofer's way to automate pipeline
// executions.
type Trigger struct {
	Name     string            `json:"name"`
	Label    string            `json:"label"`
	Settings map[string]string `json:"settings"`
}

// Attach the pipeline to one of Gofer's triggers. Triggers are Gofer's way to automate pipeline
// executions.
//
// You can use `gofer triggers list` to view current triggers available.
func NewTrigger(name, label string) *TriggerWrapper {
	return &TriggerWrapper{
		Trigger{
			Name:  name,
			Label: label,
		},
	}
}

// Add a single setting. Settings allows you to control the behavior of a trigger.
// Make sure to read the trigger's readme in order to understand which settings and their
// associated values are accepted.
func (p *TriggerWrapper) Setting(key, value string) *TriggerWrapper {
	p.Trigger.Settings[fmt.Sprintf("GOFER_PLUGIN_PARAM_%s", strings.ToUpper(key))] = value
	return p
}

// Add multiple settings. Settings allows you to control the behavior of a trigger.
// Make sure to read the trigger's readme in order to understand which settings and their
// associated values are accepted.
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
