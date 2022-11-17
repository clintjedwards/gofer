// The models package contains the core objects needed by Gofer and friends.
//
// A note about changes to this package:
//
// This package can be somewhat brittle as Go does not alert on missing struct
// fields and converting things to proto is done manually. The combination of
// these two things means that unfortunately these models may connect to other
// models(namely the proto models, but also the sdk and potentially others)
// in ways that might not be obvious and changes to them might break things
// in ways that are hard to set up testing for. Testing would need to
// heavily use reflection to prevent breakages and for now I don't have the
// time.
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

func (t *RegistryAuth) ToProto() *proto.RegistryAuth {
	if t == nil {
		return nil
	}

	return &proto.RegistryAuth{
		User: t.User,
		Pass: t.Pass,
	}
}

func (t *RegistryAuth) FromProto(pb *proto.RegistryAuth) {
	if pb == nil {
		return
	}

	if t == nil {
		t = &RegistryAuth{}
	}

	t.User = pb.User
	t.Pass = pb.Pass
}
