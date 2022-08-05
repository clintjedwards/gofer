package models

import (
	"time"

	proto "github.com/clintjedwards/gofer/proto/go"
)

type TokenKind string

const (
	TokenKindUnknown    TokenKind = "UNKNOWN"
	TokenKindManagement TokenKind = "MANAGEMENT"
	TokenKindClient     TokenKind = "CLIENT"
)

// Token is a representation of the API key, belonging to an owner.
type Token struct {
	Hash       string            `json:"hash"`       // SHA-256 hash of the secret ID.
	Created    int64             `json:"created"`    // Create time in epoch millisecond
	Kind       TokenKind         `json:"kind"`       // The type of token. Management tokens are essentially root.
	Namespaces []string          `json:"namespaces"` // List of namespaces this token has access to.
	Metadata   map[string]string `json:"metadata"`   // Extra information about this token in label form.
}

func NewToken(hash string, kind TokenKind, namespaces []string, metadata map[string]string) *Token {
	return &Token{
		Hash:       hash,
		Created:    time.Now().UnixMilli(),
		Kind:       kind,
		Namespaces: namespaces,
		Metadata:   metadata,
	}
}

func (t *Token) ToProto() *proto.Token {
	return &proto.Token{
		Created:    t.Created,
		Kind:       proto.Token_Kind(proto.Token_Kind_value[string(t.Kind)]),
		Namespaces: t.Namespaces,
		Metadata:   t.Metadata,
	}
}
