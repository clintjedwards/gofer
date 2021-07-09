package models

import (
	"time"

	"github.com/clintjedwards/gofer/proto"
)

type DockerRegistryAuth struct {
	Registry string `json:"registry" storm:"id"`
	User     string `json:"user"`
	Pass     []byte `json:"pass"`
	Created  int64  `json:"created"`
}

func NewDockerRegistryAuth(registry, user string, pass []byte) *DockerRegistryAuth {
	return &DockerRegistryAuth{
		Created:  time.Now().UnixMilli(),
		Registry: registry,
		User:     user,
		Pass:     pass,
	}
}

func (d *DockerRegistryAuth) ToProto() *proto.DockerRegistryAuth {
	return &proto.DockerRegistryAuth{
		Created:  d.Created,
		Registry: d.Registry,
		User:     d.User,
	}
}
