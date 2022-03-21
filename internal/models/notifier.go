package models

import (
	"github.com/clintjedwards/gofer/proto"
)

type Notifier struct {
	// Kind is a unique identifier for the notifier usually in plain english.
	Kind          string            `json:"kind" storm:"id,unique"`
	Image         string            `json:"image"`
	Documentation string            `json:"documentation"` // The documentation link for this specific notifier.
	RegistryAuth  RegistryAuth      `json:"-"`
	EnvVars       map[string]string `json:"-"`
}

func (t *Notifier) ToProto() *proto.Notifier {
	return &proto.Notifier{
		Kind:          t.Kind,
		Image:         t.Image,
		Documentation: t.Documentation,
	}
}
