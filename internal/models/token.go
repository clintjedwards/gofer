package models

import (
	"encoding/json"
	"fmt"
	"strconv"
	"time"

	"github.com/clintjedwards/gofer/internal/storage"
	"github.com/rs/zerolog/log"
)

type TokenType string

const (
	TokenTypeUnknown    TokenType = "UNKNOWN"
	TokenTypeManagement TokenType = "MANAGEMENT"
	TokenTypeClient     TokenType = "CLIENT"
)

type Token struct {
	ID         string            `json:"id" example:"de3foi" doc:"The unique identifier for the token"`
	Hash       string            `json:"-" hidden:"true"`
	Created    uint64            `json:"created" example:"1712433802634" doc:"Time in epoch milliseconds since token was created."`
	TokenType  TokenType         `json:"token_type" example:"MANAGEMENT" doc:"The type of the token. Management tokens are essentially root."`
	Namespaces []string          `json:"namespaces" example:"[\"default\"]" doc:"List of namespaces this token has access to, strings in this list can be a regex."`
	Metadata   map[string]string `json:"metadata" example:"{\"created_by\": \"me\"}" doc:"Extra information about this token in label form."`
	Expires    uint64            `json:"expires" example:"1712433802634" doc:"Time in epoch milliseconds when the token would expire."`
	Disabled   bool              `json:"disabled" example:"false" doc:"If the token is inactive or not; disabled tokens cannot be used."`
}

func NewToken(hash string, kind TokenType, namespaces []string, metadata map[string]string, expiry time.Duration) *Token {
	now := time.Now()
	expires := now.Add(expiry)

	id := generateID(12)

	return &Token{
		ID:         id,
		Hash:       hash,
		Created:    uint64(time.Now().UnixMilli()),
		TokenType:  kind,
		Namespaces: namespaces,
		Metadata:   metadata,
		Expires:    uint64(expires.UnixMilli()),
		Disabled:   false,
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
		ID:         t.ID,
		Hash:       t.Hash,
		Created:    fmt.Sprint(t.Created),
		Kind:       string(t.TokenType),
		Namespaces: string(namespaces),
		Metadata:   string(metadata),
		Expires:    fmt.Sprint(t.Expires),
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

	created, err := strconv.ParseUint(s.Created, 10, 64)
	if err != nil {
		log.Fatal().Err(err).Msg("error in translating from storage")
	}

	expires, err := strconv.ParseUint(s.Expires, 10, 64)
	if err != nil {
		log.Fatal().Err(err).Msg("error in translating from storage")
	}

	t.ID = s.ID
	t.Hash = s.Hash
	t.Created = created
	t.TokenType = TokenType(s.Kind)
	t.Namespaces = namespaces
	t.Metadata = metadata
	t.Expires = expires
	t.Disabled = s.Disabled
}
