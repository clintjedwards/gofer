package models

import (
	"encoding/json"

	"github.com/clintjedwards/gofer/events"
	"github.com/clintjedwards/gofer/internal/storage"
	proto "github.com/clintjedwards/gofer/proto/go"
	"github.com/rs/zerolog/log"
)

type Event struct {
	events.Event
}

func FromEvent(event *events.Event) Event {
	return Event{*event}
}

func (e *Event) ToProto() (*proto.Event, error) {
	details, err := json.Marshal(e.Details)
	if err != nil {
		return nil, err
	}

	return &proto.Event{
		Id:      e.ID,
		Type:    string(e.Type),
		Details: string(details),
		Emitted: e.Emitted,
	}, nil
}

func (e *Event) ToStorage() *storage.Event {
	details, err := json.Marshal(e.Details)
	if err != nil {
		log.Fatal().Err(err).Msg("could not (un)marshal from storage")
	}

	return &storage.Event{
		ID:      e.ID,
		Type:    string(e.Type),
		Details: string(details),
		Emitted: e.Emitted,
	}
}

func (e *Event) FromStorage(evt *storage.Event) {
	detail := events.EventTypeMap[events.EventType(evt.Type)]

	err := json.Unmarshal([]byte(evt.Details), &detail)
	if err != nil {
		log.Fatal().Err(err).Msg("could not (un)marshal from storage")
	}

	e.ID = evt.ID
	e.Type = events.EventType(evt.Type)
	e.Details = detail
	e.Emitted = evt.Emitted
}
