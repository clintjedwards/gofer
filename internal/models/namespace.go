package models

import (
	"time"

	"github.com/clintjedwards/gofer/proto"
)

// Namespace represents a division of pipelines. Normally it is used to divide teams or logically different
// sections of workloads. It is the highest level unit.
type Namespace struct {
	ID          string `json:"id" storm:"id"` // Unique identifier; user defined.
	Name        string `json:"name"`          // Humanized name; great for reading from UIs.
	Description string `json:"description"`   // Short description on what name space is used for.
	Created     int64  `json:"created"`       // The creation time in epoch milli.
	Deleted     int64  `json:"deleted"`       // The deletion time in epoch milli; 0 if not deleted.
	// Tokens      []string `json:"tokens"`      // List of tokens that have access to this namespace.
}

func NewNamespace(id, name, description string) *Namespace {
	newNamespace := &Namespace{
		ID:          id,
		Name:        name,
		Description: description,
		Created:     time.Now().UnixMilli(),
		Deleted:     0,
	}

	return newNamespace
}

func (n *Namespace) ToProto() *proto.Namespace {
	return &proto.Namespace{
		Id:          n.ID,
		Name:        n.Name,
		Description: n.Description,
		Created:     n.Created,
		Deleted:     n.Deleted,
	}
}

func (n *Namespace) FromProto(proto *proto.Namespace) {
	n.ID = proto.Id
	n.Name = proto.Name
	n.Description = proto.Description
	n.Created = proto.Created
}
