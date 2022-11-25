// Package models contains the core objects(otherwise called the domain models) needed by Gofer internally.
//
// This package exists to provide easy to use in-memory objects for concepts Gofer has to work with.
// For example, the concept of a Pipeline might make sense to the database as 3-4 different
// models/structs containing different extensions that a pipeline might be made out of, but to the API a pipeline might
// be easier represented as one single entity containing all those different parts. This might make it easier
// to pass around and work with in general.
//
// The next question that might be asked is why separate these models at all? Why not have one struct that we ride
// all the way to the top? The answer to that is the separation of domain models from DB models and in general any
// other model that has a contract with something else is usually a good thing. This loose coupling enables a lot of
// good practices as the code base gets more complex overtime. Two examples:
//  1. We may want to use simplier names for our database naming in order to save space, but more descriptive names
//     for our domain layer.
//  2. We may want to change something in our database layer and might not want to go through the trouble of having
//     to also change the API contract with outside users. Changes on that level can be extremely expensive in terms
//     of engineering time/complexcity etc.
//
// You can read more about this here: https://threedots.tech/post/common-anti-patterns-in-go-web-applications/
//
// Because this package contains the domain models, it effectively acts as the middle package between the
// internal storage layer and the external protobuf layer. The general flow goes like this:
//
//	Protobuf from API calls <-> models internally <-> backend storage.
//
// Something to keep in mind when making changes to this package:
//
// This package can be somewhat brittle as Go does not alert on missing struct fields and converting things to
// proto/storage is done manually. The combination of these two things means that unfortunately these models
// may connect to other models in ways that might not be obvious and changes to them might break things
// in ways that are hard to set up testing for. Testing would need to heavily use reflection to prevent
// breakages and for now I don't have the time. The real solution would probably be something involving code
// generation, but I don't have a good answer for that either.
package models

import (
	"encoding/json"

	proto "github.com/clintjedwards/gofer/proto/go"
	"github.com/rs/zerolog/log"
)

type VariableSource string

const (
	VariableSourceUnknown        VariableSource = "UNKNOWN"
	VariableSourcePipelineConfig VariableSource = "PIPELINE_CONFIG"
	VariableSourceSystem         VariableSource = "SYSTEM"
	VariableSourceRunOptions     VariableSource = "RUN_OPTIONS"
	VariableSourceExtension      VariableSource = "EXTENSION"
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

func (t *RegistryAuth) FromStorage(jsonStr string) {
	if jsonStr == "" {
		return
	}

	var registryAuth RegistryAuth

	err := json.Unmarshal([]byte(jsonStr), &registryAuth)
	if err != nil {
		log.Fatal().Err(err).Msg("error in translating from storage")
	}

	t.User = registryAuth.User
	t.Pass = registryAuth.Pass
}

func (t *RegistryAuth) ToStorage() string {
	jsonStr, err := json.Marshal(t)
	if err != nil {
		log.Fatal().Err(err).Msg("error in translating to storage")
	}

	return string(jsonStr)
}
