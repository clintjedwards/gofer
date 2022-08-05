package models

import (
	"time"

	proto "github.com/clintjedwards/gofer/proto/go"
)

// Namespace represents a division of pipelines. Normally it is used to divide teams or logically different
// sections of workloads. It is the highest level unit.
type Namespace struct {
	ID          string `json:"id"`          // Unique identifier; user defined.
	Name        string `json:"name"`        // Humanized name; great for reading from UIs.
	Description string `json:"description"` // Short description on what namespace is used for.
	Created     int64  `json:"created"`     // The creation time in epoch milli.
	Modified    int64  `json:"modified"`    // The modified time in epoch milli;
}

func NewNamespace(id, name, description string) *Namespace {
	newNamespace := &Namespace{
		ID:          id,
		Name:        name,
		Description: description,
		Created:     time.Now().UnixMilli(),
		Modified:    time.Now().UnixMilli(),
	}

	return newNamespace
}

func (n *Namespace) ToProto() *proto.Namespace {
	return &proto.Namespace{
		Id:          n.ID,
		Name:        n.Name,
		Description: n.Description,
		Created:     n.Created,
		Modified:    n.Modified,
	}
}

func (n *Namespace) FromProto(proto *proto.Namespace) {
	n.ID = proto.Id
	n.Name = proto.Name
	n.Description = proto.Description
	n.Created = proto.Created
	n.Modified = proto.Modified
}
