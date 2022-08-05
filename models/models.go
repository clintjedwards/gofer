package models

import (
	proto "github.com/clintjedwards/gofer/proto/go"
)

type VariableSource string

const (
	VariableSourceUnknown        VariableSource = "UNKNOWN"
	VariableSourcePipelineConfig VariableSource = "PIPELINE_CONFIG"
	VariableSourceSystem         VariableSource = "SYSTEM"
	VariableSourceRunOptions     VariableSource = "RUN_OPTIONS"
	VariableSourceTrigger        VariableSource = "TRIGGER"
)

// A variable is a key value pair that is used either in a run or task level.
// The variable is inserted as an environment variable to an eventual task run.
// It can be owned by different parts of the system which control where
// the potentially sensitive variables might show up.
type Variable struct {
	Key    string         `json:"key"`
	Value  string         `json:"value"`
	Source VariableSource `json:"source"` // Where the variable originated from
}

func (v *Variable) ToProto() *proto.Variable {
	return &proto.Variable{
		Key:    v.Key,
		Value:  v.Value,
		Source: string(v.Source),
	}
}

func (v *Variable) FromProto(proto *proto.Variable) {
	v.Key = proto.Key
	v.Value = proto.Value
	v.Source = VariableSource(proto.Source)
}

type RegistryAuth struct {
	User string `json:"user"`
	Pass string `json:"pass"`
}
