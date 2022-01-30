package eventbus

import (
	"crypto/rand"
	"errors"
	"fmt"
	"sync"
	"time"

	"github.com/clintjedwards/gofer/internal/models"
	"github.com/clintjedwards/gofer/internal/storage"
	"github.com/rs/zerolog/log"
	zlog "github.com/rs/zerolog/log"
)

// Duplicate events are possible

var (
	ErrEventKindNotFound = errors.New("eventbus: event kind does not exist")
	ErrEventNotFound     = errors.New("eventbus: event could not be found")
)

// Subscription is a representation of a new Subscription to a certain topic.
type Subscription struct {
	id     string
	kind   models.EventType
	Events chan models.Event
}

func generateID(length int) string {
	b := make([]byte, length)
	_, _ = rand.Read(b)
	return fmt.Sprintf("%x", b)
}

func newSubscriber(kind models.EventType, channel chan models.Event) Subscription {
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
	storage     storage.Engine
	retention   time.Duration
	subscribers map[models.EventType][]Subscription // channel tracking per subscriber
}

// New create a new instance of the eventbus and populates the log from disk.
func New(storage storage.Engine, retention time.Duration, pruneInterval time.Duration) (*EventBus, error) {
	eb := &EventBus{
		storage:     storage,
		retention:   retention,
		subscribers: map[models.EventType][]Subscription{},
	}

	go func() {
		for {
			eb.pruneEvents()
			time.Sleep(pruneInterval)
		}
	}()

	for eventType := range models.EventMap {
		eb.subscribers[eventType] = []Subscription{}
	}

	return eb, nil
}

// Subscribe returns a channel in which the caller can listen for all events of a particular type.
func (eb *EventBus) Subscribe(kind models.EventType) (Subscription, error) {
	eb.mu.Lock()
	defer eb.mu.Unlock()

	listeners, exists := eb.subscribers[kind]
	if !exists {
		return Subscription{}, fmt.Errorf("event kind %q not found: %w", kind, ErrEventKindNotFound)
	}

	newSub := newSubscriber(kind, make(chan models.Event, 10))

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
}

// Publish allows caller to emit a new event to the eventbus.
func (eb *EventBus) Publish(evt models.Event) {
	eb.mu.Lock()
	defer eb.mu.Unlock()

	err := eb.storage.AddEvent(storage.AddEventRequest{Event: evt})
	if err != nil {
		log.Error().Err(err).Msg("could not add event to storage")
	}

	listeners, exists := eb.subscribers[evt.GetKind()]
	if !exists {
		zlog.Error().Err(ErrEventKindNotFound).Msgf("event kind %q not found", evt.GetKind())
		return
	}

	anyListeners, exists := eb.subscribers[models.AnyEvent]
	if !exists {
		zlog.Error().Err(ErrEventKindNotFound).Msgf("event kind %q not found", models.AnyEvent)
		return
	}

	for _, anyListener := range anyListeners {
		go func(anyListener Subscription) {
			anyListener.Events <- evt
		}(anyListener)
	}

	for _, subscription := range listeners {
		go func(subscription Subscription) {
			subscription.Events <- evt
		}(subscription)
	}
}

// GetAll returns all events. Returns events from oldest to newest unless reverse parameter is set.
func (eb *EventBus) GetAll(reverse bool) <-chan models.Event {
	events := make(chan models.Event, 10)

	go func() {
		offset := 0

		for {
			eventList, err := eb.storage.GetAllEvents(storage.GetAllEventsRequest{
				Offset:  offset,
				Limit:   10,
				Reverse: reverse,
			})
			if err != nil {
				log.Error().Err(err).Msg("could not get events")
				close(events)
				return
			}

			if len(eventList) == 0 {
				close(events)
				return
			}

			for _, event := range eventList {
				events <- event
			}

			offset += 10
		}
	}()

	return events
}

// Get returns a single event by id. Returns a eventbus.ErrEventNotFound if the event could not be located.
func (eb *EventBus) Get(id int64) (models.Event, error) {
	event, err := eb.storage.GetEvent(storage.GetEventRequest{
		ID: id,
	})
	if err != nil {
		if errors.Is(err, storage.ErrEntityNotFound) {
			return nil, ErrEventNotFound
		}
		return nil, err
	}

	return event, nil
}

func (eb *EventBus) pruneEvents() {
	offset := 0

	totalPruned := 0

	for {
		events, err := eb.storage.GetAllEvents(storage.GetAllEventsRequest{
			Offset:  offset,
			Limit:   50,
			Reverse: false,
		})
		if err != nil {
			log.Error().Err(err).Msg("could not get events from storage")
			return
		}

		for _, event := range events {
			if isPastCutDate(event, eb.retention) {
				log.Debug().Int64("event_id", event.GetID()).Dur("retention", eb.retention).
					Int64("emitted", event.GetEmitted()).
					Int64("current_time", time.Now().UnixMilli()).Msg("removed event past retention")
				totalPruned++
				err := eb.storage.DeleteEvent(storage.DeleteEventRequest{
					ID: event.GetID(),
				})
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

func isPastCutDate(event models.Event, limit time.Duration) bool {
	cut := time.Now().Add(-limit) // Even though this function says add, we're actually subtracting time.

	return event.GetEmitted() < cut.UnixMilli()
}
