package eventbus

import (
	"crypto/rand"
	"errors"
	"fmt"
	"sync"
	"time"

	"github.com/clintjedwards/gofer/events"
	"github.com/clintjedwards/gofer/internal/models"
	"github.com/clintjedwards/gofer/internal/storage"
	"github.com/rs/zerolog/log"
)

// Duplicate events are possible

var (
	ErrEventKindNotFound = errors.New("eventbus: event kind does not exist")
	ErrEventNotFound     = errors.New("eventbus: event could not be found")
)

// Subscription is a representation of a new Subscription to a certain topic.
type Subscription struct {
	id     string
	kind   events.EventType
	Events chan events.Event
}

func generateID(length int) string {
	b := make([]byte, length)
	_, _ = rand.Read(b)
	return fmt.Sprintf("%x", b)
}

func newSubscriber(kind events.EventType, channel chan events.Event) Subscription {
	return Subscription{
		id:     generateID(5),
		kind:   kind,
		Events: channel,
	}
}

// EventBus is a central handler for all things related to events within the application.
type EventBus struct {
	mu sync.Mutex // lock for concurrency safety.

	// storage layer for persistance. Events are capped at a particular size.
	storage     storage.DB
	retention   time.Duration
	subscribers map[events.EventType][]Subscription // channel tracking per subscriber
}

// New create a new instance of the eventbus and populates the log from disk.
func New(storage storage.DB, retention time.Duration, pruneInterval time.Duration) (*EventBus, error) {
	eb := &EventBus{
		storage:     storage,
		retention:   retention,
		subscribers: map[events.EventType][]Subscription{},
	}

	go func() {
		for {
			eb.pruneEvents()
			time.Sleep(pruneInterval)
		}
	}()

	for eventKind := range events.EventTypeMap {
		eb.subscribers[eventKind] = []Subscription{}
	}
	eb.subscribers[events.EventTypeAny] = []Subscription{}

	return eb, nil
}

// Subscribe returns a channel in which the caller can listen for all events of a particular type.
func (eb *EventBus) Subscribe(kind events.EventType) (Subscription, error) {
	eb.mu.Lock()
	defer eb.mu.Unlock()

	listeners, exists := eb.subscribers[kind]
	if !exists {
		return Subscription{}, fmt.Errorf("event kind %q not found: %w", kind, ErrEventKindNotFound)
	}

	newSub := newSubscriber(kind, make(chan events.Event, 10))

	listeners = append(listeners, newSub)
	eb.subscribers[kind] = listeners

	return newSub, nil
}

func (eb *EventBus) Unsubscribe(sub Subscription) {
	eb.mu.Lock()
	defer eb.mu.Unlock()

	listeners, exists := eb.subscribers[sub.kind]
	if !exists {
		return
	}

	for index, listener := range listeners {
		if listener.id != sub.id {
			continue
		}

		listeners[index] = listeners[len(listeners)-1]
		listeners = listeners[:len(listeners)-1]
	}

	eb.subscribers[sub.kind] = listeners
}

// Publish allows caller to emit a new event to the eventbus. Might block until it can publish to all listeners.
func (eb *EventBus) Publish(evt events.EventTypeDetails) int64 {
	eventRaw := events.NewEvent(evt)
	event := models.FromEvent(eventRaw)

	id, err := eb.storage.InsertEvent(eb.storage, event.ToStorage())
	if err != nil {
		log.Error().Err(err).Msg("could not add event to storage")
	}

	event.ID = id

	eb.mu.Lock()
	defer eb.mu.Unlock()

	listeners, exists := eb.subscribers[evt.Kind()]
	if !exists {
		log.Error().Err(ErrEventKindNotFound).Msgf("event type %q not found; This usually means that an event is missing from the EventTypeMap object.", evt.Kind())
		return 0
	}

	anyListeners, exists := eb.subscribers[events.EventTypeAny]
	if !exists {
		log.Error().Err(ErrEventKindNotFound).Msgf("event type %q not found", events.EventTypeAny)
		return 0
	}

	// It is naive to think that we can use go-routines to avoid blocking here.
	// Doing so leads to races where an event published after might actually be published before another.
	// This is due to goroutine scheduling.
	for _, anyListener := range anyListeners {
		anyListener.Events <- event.Event
	}

	for _, subscription := range listeners {
		subscription.Events <- event.Event
	}

	log.Debug().Interface("event", event).Msg("new event published")

	return id
}

// GetAll returns all events. Returns events from oldest to newest unless reverse parameter is set.
func (eb *EventBus) GetAll(reverse bool) <-chan events.Event {
	events := make(chan events.Event, 10)

	go func() {
		offset := 0

		for {
			eventList, err := eb.storage.ListEvents(eb.storage, offset, 10, reverse)
			if err != nil {
				log.Error().Err(err).Msg("could not get events")
				close(events)
				return
			}

			if len(eventList) == 0 {
				close(events)
				return
			}

			for _, rawEvent := range eventList {
				event := models.Event{}
				event.FromStorage(&rawEvent)

				events <- event.Event
			}

			offset += 10
		}
	}()

	return events
}

// Get returns a single event by id. Returns a eventbus.ErrEventNotFound if the event could not be located.
func (eb *EventBus) Get(id int64) (events.Event, error) {
	rawEvent, err := eb.storage.GetEvent(eb.storage, id)
	if err != nil {
		if errors.Is(err, storage.ErrEntityNotFound) {
			return events.Event{}, ErrEventNotFound
		}
		return events.Event{}, err
	}

	event := models.Event{}
	event.FromStorage(&rawEvent)

	return event.Event, nil
}

func (eb *EventBus) pruneEvents() {
	offset := 0

	totalPruned := 0

	for {
		events, err := eb.storage.ListEvents(eb.storage, offset, 50, false)
		if err != nil {
			log.Error().Err(err).Msg("could not get events from storage")
			return
		}

		for _, rawEvent := range events {
			event := models.Event{}
			event.FromStorage(&rawEvent)

			if isPastCutDate(event.Event, eb.retention) {
				log.Debug().Int64("event_id", event.ID).Dur("retention", eb.retention).
					Int64("emitted", event.Emitted).
					Int64("current_time", time.Now().UnixMilli()).Msg("removed event past retention")
				totalPruned++
				err := eb.storage.DeleteEvent(eb.storage, event.ID)
				if err != nil {
					log.Error().Err(err).Msg("could not delete event")
					return
				}
				continue
			}
		}

		if len(events) != 50 {
			if totalPruned > 0 {
				log.Info().Dur("retention", eb.retention).Int("total", totalPruned).Msg("pruned old events")
			}
			return
		}

		offset += len(events)
	}
}

func isPastCutDate(event events.Event, limit time.Duration) bool {
	cut := time.Now().Add(-limit) // Even though this function says add, we're actually subtracting time.

	return event.Emitted < cut.UnixMilli()
}
