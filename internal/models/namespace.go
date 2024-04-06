package models

import (
	"fmt"
	"strconv"
	"time"

	"github.com/clintjedwards/gofer/internal/storage"
	"github.com/rs/zerolog/log"
)

// Namespace represents a division of pipelines. Normally it is used to divide teams or logically different
// sections of workloads. It is the highest level unit.
type Namespace struct {
	ID          string `json:"id" example:"default" doc:"Unique identifier of the target namespace"`
	Name        string `json:"name" example:"Default" doc:"Humanized name for the namespace"`
	Description string `json:"description" example:"some thoughts about the namespace" doc:"Short description on what the namespace is used for"`
	Created     uint64 `json:"created" example:"1712433802634" doc:"Time object was created in epoch milliseconds"`
	Modified    uint64 `json:"modified" example:"1712433802634" doc:"Time object was modified in epoch milliseconds"`
}

func NewNamespace(id, name, description string) *Namespace {
	newNamespace := &Namespace{
		ID:          id,
		Name:        name,
		Description: description,
		Created:     uint64(time.Now().UnixMilli()),
		Modified:    uint64(time.Now().UnixMilli()),
	}

	return newNamespace
}

func (n *Namespace) ToStorage() *storage.Namespace {
	return &storage.Namespace{
		ID:          n.ID,
		Name:        n.Name,
		Description: n.Description,
		Created:     fmt.Sprint(n.Created),
		Modified:    fmt.Sprint(n.Modified),
	}
}

func (n *Namespace) FromStorage(sn *storage.Namespace) {
	created, err := strconv.ParseUint(sn.Created, 10, 64)
	if err != nil {
		log.Fatal().Err(err).Msg("error in translating from storage")
	}

	modified, err := strconv.ParseUint(sn.Modified, 10, 64)
	if err != nil {
		log.Fatal().Err(err).Msg("error in translating from storage")
	}

	n.ID = sn.ID
	n.Name = sn.Name
	n.Description = sn.Description
	n.Created = created
	n.Modified = modified
}
