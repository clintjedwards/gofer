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
	Expires    int64             `json:"expiry"`     // When the token would expire.
	Disabled   bool              `json:"disabled"`   // Disabled tokens cannot be used.
}

func NewToken(hash string, kind TokenKind, namespaces []string, metadata map[string]string, expiry time.Duration) *Token {
	now := time.Now()
	expires := now.Add(expiry)

	return &Token{
		Hash:       hash,
		Created:    time.Now().UnixMilli(),
		Kind:       kind,
		Namespaces: namespaces,
		Metadata:   metadata,
		Expires:    expires.UnixMilli(),
		Disabled:   false,
	}
}

func (t *Token) ToProto() *proto.Token {
	return &proto.Token{
		Created:    t.Created,
		Kind:       proto.Token_Kind(proto.Token_Kind_value[string(t.Kind)]),
		Namespaces: t.Namespaces,
		Metadata:   t.Metadata,
		Expires:    t.Expires,
		Disabled:   t.Disabled,
	}
}

func (t *Token) FromProto(proto *proto.Token) {
	t.Created = proto.Created
	t.Kind = TokenKind(proto.Kind.String())
	t.Namespaces = proto.Namespaces
	t.Metadata = proto.Metadata
	t.Expires = proto.Expires
	t.Disabled = proto.Disabled
}
