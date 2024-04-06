package models

import (
	"encoding/json"
	"fmt"
	"strconv"

	"github.com/clintjedwards/gofer/events"
	"github.com/clintjedwards/gofer/internal/storage"
	"github.com/rs/zerolog/log"
)

type Event struct {
	events.Event
}

func FromEvent(event *events.Event) Event {
	return Event{*event}
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
		Emitted: fmt.Sprint(e.Emitted),
	}
}

func (e *Event) FromStorage(evt *storage.Event) {
	detail := events.EventTypeMap[events.EventType(evt.Type)]

	err := json.Unmarshal([]byte(evt.Details), &detail)
	if err != nil {
		log.Fatal().Err(err).Msg("could not (un)marshal from storage")
	}

	emitted, err := strconv.ParseUint(evt.Emitted, 10, 64)
	if err != nil {
		log.Fatal().Err(err).Msg("error in translating from storage")
	}

	e.ID = evt.ID
	e.Type = events.EventType(evt.Type)
	e.Details = detail
	e.Emitted = emitted
}
