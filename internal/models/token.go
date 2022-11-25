package models

import (
	"encoding/json"
	"time"

	"github.com/clintjedwards/gofer/internal/storage"
	proto "github.com/clintjedwards/gofer/proto/go"
	"github.com/rs/zerolog/log"
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

func (t *Token) ToStorage() *storage.Token {
	namespaces, err := json.Marshal(t.Namespaces)
	if err != nil {
		log.Fatal().Err(err).Msg("error in translating to storage")
	}

	metadata, err := json.Marshal(t.Metadata)
	if err != nil {
		log.Fatal().Err(err).Msg("error in translating to storage")
	}

	return &storage.Token{
		Hash:       t.Hash,
		Created:    t.Created,
		Kind:       string(t.Kind),
		Namespaces: string(namespaces),
		Metadata:   string(metadata),
		Expires:    t.Expires,
		Disabled:   t.Disabled,
	}
}

func (t *Token) FromStorage(s *storage.Token) {
	var namespaces []string
	err := json.Unmarshal([]byte(s.Namespaces), &namespaces)
	if err != nil {
		log.Fatal().Err(err).Msg("error in translating from storage")
	}

	var metadata map[string]string
	err = json.Unmarshal([]byte(s.Metadata), &metadata)
	if err != nil {
		log.Fatal().Err(err).Msg("error in translating from storage")
	}

	t.Hash = s.Hash
	t.Created = s.Created
	t.Kind = TokenKind(s.Kind)
	t.Namespaces = namespaces
	t.Metadata = metadata
	t.Expires = s.Expires
	t.Disabled = s.Disabled
}
